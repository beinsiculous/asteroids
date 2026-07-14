//! Rock rules: screen wrapping, splitting, wave sizing, spawn placement,
//! and bullet-hit resolution. The pure functions up top carry all the math;
//! the impl below wires them to the world.

use engine_core::prelude::*;
use std::collections::HashSet;

use super::entity_position;
use crate::achievements;
use crate::constants::*;
use crate::effects;
use crate::types::*;

/// Toroidal wrap: a point that leaves the field (half extents + margin)
/// reappears on the opposite side. The margin lets an entity fully exit
/// before it re-enters.
pub(crate) fn wrap_position(pos: Vec2, half_extents: Vec2, margin: f32) -> Vec2 {
    let mut p = pos;
    let hx = half_extents.x + margin;
    let hy = half_extents.y + margin;
    if p.x > hx {
        p.x = -hx;
    } else if p.x < -hx {
        p.x = hx;
    }
    if p.y > hy {
        p.y = -hy;
    } else if p.y < -hy {
        p.y = hy;
    }
    p
}

pub(crate) fn circles_overlap(a: Vec2, ra: f32, b: Vec2, rb: f32) -> bool {
    a.distance_squared(b) <= (ra + rb) * (ra + rb)
}

/// What a shattered rock becomes: `(child size, child count)`, or `None`
/// for small rocks (they just vanish). The Ridiculous family splits large
/// rocks three ways.
pub(crate) fn split_spec(size: AsteroidSize, mode: ChaosMode) -> Option<(AsteroidSize, usize)> {
    match size {
        AsteroidSize::Large => {
            let count = if mode.is_ridiculous() {
                RIDICULOUS_LARGE_SPLIT_COUNT
            } else {
                SPLIT_COUNT
            };
            Some((AsteroidSize::Medium, count))
        }
        AsteroidSize::Medium => Some((AsteroidSize::Small, SPLIT_COUNT)),
        AsteroidSize::Small => None,
    }
}

/// Child velocity `index` of `count`: the parent direction fanned out by
/// `SPLIT_SPREAD` with hash jitter, at parent speed × `SPLIT_SPEED_MULT`
/// (± hash variance). Deterministic per seed.
pub(crate) fn split_velocity(parent_vel: Vec2, index: usize, count: usize, seed: u32) -> Vec2 {
    let base_angle = parent_vel.y.atan2(parent_vel.x);
    let fan = (index as f32 - (count as f32 - 1.0) / 2.0) * SPLIT_SPREAD;
    let jitter = (hash_f32(seed) - 0.5) * 0.3;
    let angle = base_angle + fan + jitter;
    let speed = parent_vel.length()
        * SPLIT_SPEED_MULT
        * (0.9 + 0.3 * hash_f32(seed.wrapping_add(1)));
    Vec2::new(angle.cos(), angle.sin()) * speed
}

/// Large rocks in wave `wave`: grows by one per wave up to the cap, plus
/// one extra in the Ridiculous family.
pub(crate) fn wave_asteroid_count(wave: u32, mode: ChaosMode) -> usize {
    let base = (WAVE_BASE_ASTEROIDS + wave.saturating_sub(1) as usize).min(WAVE_MAX_ASTEROIDS);
    base + if mode.is_ridiculous() { RIDICULOUS_EXTRA_ASTEROIDS } else { 0 }
}

/// A point on the window border rectangle, parametrized by t in [0, 1)
/// walking the perimeter clockwise from the top-left corner.
fn border_point(t: f32) -> Vec2 {
    let hw = WIN_W / 2.0;
    let hh = WIN_H / 2.0;
    let d = t.rem_euclid(1.0) * 2.0 * (WIN_W + WIN_H);
    if d < WIN_W {
        Vec2::new(-hw + d, hh)
    } else if d < WIN_W + WIN_H {
        Vec2::new(hw, hh - (d - WIN_W))
    } else if d < 2.0 * WIN_W + WIN_H {
        Vec2::new(hw - (d - WIN_W - WIN_H), -hh)
    } else {
        Vec2::new(-hw, -hh + (d - 2.0 * WIN_W - WIN_H))
    }
}

/// Hash-picked border spawn point, never within `SAFE_SPAWN_DIST` of the
/// ship. Opposite perimeter points on this window are always >= 600 px
/// apart, so the half-perimeter fallback resolves deterministically.
pub(crate) fn spawn_position(seed: u32, ship_pos: Vec2) -> Vec2 {
    let t = hash_f32(seed);
    let first = border_point(t);
    if first.distance(ship_pos) >= SAFE_SPAWN_DIST {
        first
    } else {
        border_point(t + 0.5)
    }
}

/// Hash-picked drift speed within the size class's range; the Insane
/// family multiplies it.
pub(crate) fn asteroid_speed(size: AsteroidSize, seed: u32, mode: ChaosMode) -> f32 {
    let (min, max) = size.speed_range();
    let speed = min + (max - min) * hash_f32(seed);
    speed * if mode.is_insane() { INSANE_SPEED_MULT } else { 1.0 }
}

impl AsteroidsGame {
    /// Match this frame's started contacts against live bullets and rocks.
    /// Pairs are collected first so each bullet and each rock resolves at
    /// most once per frame (a bullet grazing two rocks kills only one).
    pub(crate) fn resolve_bullet_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let mut used_bullets: HashSet<EntityId> = HashSet::new();
        let mut used_rocks: HashSet<EntityId> = HashSet::new();
        let mut hits: Vec<(EntityId, EntityId)> = Vec::new();

        for c in collisions.iter().filter(|c| c.event.started) {
            for &bullet in &self.bullets {
                if used_bullets.contains(&bullet) {
                    continue;
                }
                let Some(rock) = self.asteroids.iter().find(|r| {
                    !used_rocks.contains(&r.entity) && c.event.involves(bullet, r.entity)
                }) else {
                    continue;
                };
                used_bullets.insert(bullet);
                used_rocks.insert(rock.entity);
                hits.push((bullet, rock.entity));
                break;
            }
        }

        for (bullet, rock) in hits {
            self.bullets.retain(|&b| b != bullet);
            self.physics.destroy_entity(ctx.world, bullet);
            self.shatter_asteroid(ctx, rock);
        }
    }

    /// Destroy a rock: effects, scoring, skill achievements, and the split
    /// into smaller children (offset outward so the solid bodies don't
    /// spawn deeply interpenetrated).
    pub(crate) fn shatter_asteroid(&mut self, ctx: &mut GameContext, entity: EntityId) {
        let Some(idx) = self.asteroids.iter().position(|r| r.entity == entity) else { return };
        let rock = self.asteroids.swap_remove(idx);
        let pos = entity_position(ctx.world, entity).unwrap_or(Vec2::ZERO);
        let (vel, _) = self.physics.get_body_velocity(entity).unwrap_or((Vec2::ZERO, 0.0));
        self.physics.destroy_entity(ctx.world, entity);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::asteroid_break_burst(rock.size, &theme, self.tex_id));
        let (strength, radius) = GRID_IMPULSE_BREAK[rock.size.idx()];
        self.ripple_grid(pos, strength, radius);

        self.award_points(ctx.achievements, rock.size.score());
        self.shot_streak += 1;
        if self.shot_streak >= SHARPSHOOTER_TARGET {
            ctx.achievements.unlock(achievements::SHARPSHOOTER);
        }
        if let Some(ship_pos) = self.ship.and_then(|s| entity_position(ctx.world, s)) {
            if ship_pos.distance(pos) <= CLOSE_CALL_DIST {
                ctx.achievements.unlock(achievements::CLOSE_CALL);
            }
        }
        if self.play_time - self.last_kill_time <= DOUBLE_TAP_WINDOW {
            ctx.achievements.unlock(achievements::DOUBLE_TAP);
        }
        self.last_kill_time = self.play_time;

        if let Some((child_size, count)) = split_spec(rock.size, self.chaos_mode) {
            for i in 0..count {
                let seed = self.next_seed();
                let child_vel = split_velocity(vel, i, count, seed);
                let child_pos = pos + child_vel.normalize_or_zero() * rock.size.radius() * 1.2;
                let spin = (hash_f32(seed.wrapping_add(7)) - 0.5) * 2.0 * ASTEROID_SPIN_MAX;
                self.spawn_asteroid(ctx.world, child_size, child_pos, child_vel, spin);
            }
        }
    }

    /// Teleport anything that left the field to the opposite side. Writing
    /// `Transform2D.position` on a live dynamic body is a rapier teleport
    /// that preserves velocity (GPP-09) — proven by a sim test.
    pub(crate) fn wrap_entities(&mut self, ctx: &mut GameContext) {
        let half = Vec2::new(WIN_W / 2.0, WIN_H / 2.0);
        let mut targets: Vec<(EntityId, f32)> = Vec::new();
        if let Some(ship) = self.ship {
            targets.push((ship, SHIP_SIZE));
        }
        targets.extend(self.asteroids.iter().map(|r| (r.entity, r.size.radius())));
        targets.extend(self.bullets.iter().map(|&b| (b, BULLET_RADIUS)));

        for (entity, margin) in targets {
            if let Some(t) = ctx.world.get_mut::<Transform2D>(entity) {
                let wrapped = wrap_position(t.position, half, margin);
                if wrapped != t.position {
                    t.position = wrapped;
                }
            }
        }
    }

    /// Once the field is clear, breathe for `WAVE_CLEAR_DELAY`, then start
    /// the next (bigger) wave.
    pub(crate) fn check_wave_clear(&mut self, ctx: &mut GameContext) {
        if !self.asteroids.is_empty() {
            return;
        }
        if self.wave_clear_timer <= 0.0 {
            self.wave_clear_timer = WAVE_CLEAR_DELAY;
            self.waves_without_death += 1;
            if self.waves_without_death >= UNTOUCHABLE_WAVES {
                ctx.achievements.unlock(achievements::UNTOUCHABLE);
            }
            return;
        }
        self.wave_clear_timer -= ctx.delta_time;
        if self.wave_clear_timer <= 0.0 {
            self.wave_clear_timer = 0.0;
            self.wave += 1;
            if self.wave >= WAVE_MILESTONE {
                ctx.achievements.unlock(achievements::wave5_id(self.chaos_mode));
                if self.lives == 1 {
                    ctx.achievements.unlock(achievements::LIVING_ON_EDGE);
                }
            }
            let ship_pos = self.ship.and_then(|s| entity_position(ctx.world, s)).unwrap_or(Vec2::ZERO);
            self.spawn_wave(ctx.world, ship_pos);
        }
    }
}
