//! Meatieroid rules: screen wrapping, splitting, wave sizing, spawn placement,
//! and bullet-hit resolution. The pure functions up top carry all the math;
//! the impl below wires them to the world.

use engine_core::prelude::*;
use std::collections::HashSet;

use super::{entity_position, ripple_grid};
use crate::achievements;
use crate::art::*;
use crate::constants::*;
use crate::effects;
use crate::spawning::{child_machine, spawn_effect, Effect};
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

/// Child `index` of `count`'s direction in the fan, before any jitter: the parent's
/// heading spread by `SPLIT_SPREAD` between neighbours, centred on it.
fn fan_angle(parent_vel: Vec2, index: usize, count: usize) -> f32 {
    parent_vel.y.atan2(parent_vel.x) + (index as f32 - (count as f32 - 1.0) / 2.0) * SPLIT_SPREAD
}

/// Child velocity `index` of `count`: the fan's direction with hash jitter, at parent
/// speed × `SPLIT_SPEED_MULT` (± hash variance). Deterministic per seed.
pub(crate) fn split_velocity(parent_vel: Vec2, index: usize, count: usize, seed: u32) -> Vec2 {
    let jitter = (hash_f32(seed) - 0.5) * 0.3;
    let angle = fan_angle(parent_vel, index, count) + jitter;
    let speed = parent_vel.length()
        * SPLIT_SPEED_MULT
        * (0.9 + 0.3 * hash_f32(seed.wrapping_add(1)));
    Vec2::new(angle.cos(), angle.sin()) * speed
}

/// Where child `index` of `count` is born, relative to its parent's centre: along the
/// fan's unjittered direction, so two jitters can never close the fan, and far enough
/// out that adjacent siblings — `SPLIT_SPREAD` apart at the same distance — clear each
/// other's bodies by `SPLIT_SIBLING_GAP`. Solid restitution-1 bodies born overlapping
/// would pop apart.
pub(crate) fn split_offset(
    parent_vel: Vec2,
    parent: AsteroidSize,
    child: AsteroidSize,
    index: usize,
    count: usize,
) -> Vec2 {
    let clear_of_siblings = child.radius() / (SPLIT_SPREAD / 2.0).sin() + SPLIT_SIBLING_GAP;
    let distance = (SPLIT_OFFSET_PARENT_RADII * parent.radius()).max(clear_of_siblings);
    let angle = fan_angle(parent_vel, index, count);
    Vec2::new(angle.cos(), angle.sin()) * distance
}

/// Large meatieroids in wave `wave`: grows by one per wave up to the cap, plus
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

/// Smallest distance from `p` to any point in `avoid` (infinite when empty).
fn min_clearance(p: Vec2, avoid: &[Vec2]) -> f32 {
    avoid.iter().map(|&a| p.distance(a)).fold(f32::INFINITY, f32::min)
}

/// How many evenly spaced perimeter points to try before settling for the
/// roomiest. Opposite points on this window are >= 600 px apart, so with the
/// ships bunched near the center at least one candidate always clears.
const SPAWN_CANDIDATES: usize = 8;

/// Hash-picked border spawn point that clears `SAFE_SPAWN_DIST` from every
/// live ship. Walks up to `SPAWN_CANDIDATES` evenly spaced perimeter points
/// from the hashed start and returns the first that clears all ships; if none
/// does (adversarial ship placement), it returns the roomiest. Deterministic
/// per seed, and correct for both single-player and co-op avoid sets.
pub(crate) fn spawn_position(seed: u32, avoid: &[Vec2]) -> Vec2 {
    let start = hash_f32(seed);
    let mut best = border_point(start);
    let mut best_clearance = min_clearance(best, avoid);
    if best_clearance >= SAFE_SPAWN_DIST {
        return best;
    }
    for k in 1..SPAWN_CANDIDATES {
        let candidate = border_point(start + k as f32 / SPAWN_CANDIDATES as f32);
        let clearance = min_clearance(candidate, avoid);
        if clearance >= SAFE_SPAWN_DIST {
            return candidate;
        }
        if clearance > best_clearance {
            best = candidate;
            best_clearance = clearance;
        }
    }
    best
}

/// Hash-picked drift speed within the size class's range; the Insane
/// family multiplies it.
pub(crate) fn asteroid_speed(size: AsteroidSize, seed: u32, mode: ChaosMode) -> f32 {
    let (min, max) = size.speed_range();
    let speed = min + (max - min) * hash_f32(seed);
    speed * if mode.is_insane() { INSANE_SPEED_MULT } else { 1.0 }
}

/// Pure hit pairing: for each started contact, match the first unused live
/// bullet against the first unused rock the contact involves. Ships are never
/// in the candidate set — the only hittable entities are `rocks`, which is
/// what structurally guarantees no friendly fire (a bullet touching a ship
/// can never resolve as a hit). Each bullet and rock is used at most once per
/// frame. Returns `(owner_ship_index, bullet, rock)` triples.
pub(crate) fn resolve_hit_pairs(
    bullets: &[(usize, EntityId)],
    rocks: &[EntityId],
    collisions: &[CollisionData],
) -> Vec<(usize, EntityId, EntityId)> {
    let mut used_bullets: HashSet<EntityId> = HashSet::new();
    let mut used_rocks: HashSet<EntityId> = HashSet::new();
    let mut hits: Vec<(usize, EntityId, EntityId)> = Vec::new();

    for c in collisions.iter().filter(|c| c.event.started) {
        for &(ship_index, bullet) in bullets {
            if used_bullets.contains(&bullet) {
                continue;
            }
            let Some(&rock) = rocks
                .iter()
                .find(|&&r| !used_rocks.contains(&r) && c.event.involves(bullet, r))
            else {
                continue;
            };
            used_bullets.insert(bullet);
            used_rocks.insert(rock);
            hits.push((ship_index, bullet, rock));
            break;
        }
    }
    hits
}

impl AsteroidsGame {
    /// Match this frame's started contacts against live bullets and rocks,
    /// attributing each kill to the ship that owns the bullet. Pairing runs
    /// through the pure `resolve_hit_pairs` so a bullet grazing two rocks
    /// kills only one, and bullets are only ever tested against rocks.
    pub(crate) fn resolve_bullet_hits(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        let bullets: Vec<(usize, EntityId)> = self
            .ships
            .iter()
            .enumerate()
            .flat_map(|(si, s)| s.bullets.iter().map(move |&b| (si, b)))
            .collect();
        let rocks: Vec<EntityId> = self.asteroids.iter().map(|r| r.entity).collect();

        for (ship_index, bullet, rock) in resolve_hit_pairs(&bullets, &rocks, collisions) {
            self.ships[ship_index].bullets.retain(|&b| b != bullet);
            self.physics.destroy_entity(ctx.world, bullet);
            self.shatter_asteroid(ctx, ship_index, rock);
        }
    }

    /// Destroy a meatieroid hit by ship `ship_index`: its `die` plays where it was, turned
    /// as it was, beside a burst and a grid ripple; then scoring, skill achievements, and
    /// the split into smaller children, born at once. Score, streak, and the close-call
    /// check all follow the owning ship.
    pub(crate) fn shatter_asteroid(&mut self, ctx: &mut GameContext, ship_index: usize, entity: EntityId) {
        let Some(idx) = self.asteroids.iter().position(|r| r.entity == entity) else { return };
        let rock = self.asteroids.swap_remove(idx);
        let (pos, rotation) = ctx.world.get::<Transform2D>(entity)
            .map(|t| (t.position, t.rotation))
            .unwrap_or((Vec2::ZERO, 0.0));
        let (vel, _) = self.physics.get_body_velocity(entity).unwrap_or((Vec2::ZERO, 0.0));
        self.physics.destroy_entity(ctx.world, entity);

        let art = rock.size.art();
        let die = spawn_effect(ctx.world, &Effect {
            name: "Meatieroid Die",
            spec: art.sheet,
            sheet: self.sheets.meatieroid(rock.size),
            depth: art.depth,
            clip: DIE,
            position: pos,
            rotation,
        });
        self.transient_visuals.push(die);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::asteroid_break_burst(rock.size, &theme, self.tex_id));
        let (strength, radius) = GRID_IMPULSE_BREAK[rock.size.idx()];
        ripple_grid(ctx.world, pos, strength, radius);

        self.award_points(ctx.achievements, ship_index, rock.size.score());
        self.ships[ship_index].shot_streak += 1;
        if self.ships[ship_index].shot_streak >= SHARPSHOOTER_TARGET {
            ctx.achievements.unlock(achievements::SHARPSHOOTER);
        }
        let ship_pos = self.ships[ship_index].entity.and_then(|s| entity_position(ctx.world, s));
        if let Some(ship_pos) = ship_pos {
            if ship_pos.distance(pos) <= CLOSE_CALL_DIST {
                ctx.achievements.unlock(achievements::CLOSE_CALL);
            }
        }
        if self.play_time - self.last_kill_time <= DOUBLE_TAP_WINDOW {
            ctx.achievements.unlock(achievements::DOUBLE_TAP);
        }
        self.last_kill_time = self.play_time;

        self.spawn_children(ctx.world, rock.size, pos, vel);
    }

    /// Split a meatieroid of `size` at `pos` moving at `vel` into its children, if it has
    /// any: each placed along the unjittered fan (`split_offset`), flying along its
    /// jittered velocity, and born flexing.
    pub(crate) fn spawn_children(&mut self, world: &mut World, size: AsteroidSize, pos: Vec2, vel: Vec2) {
        let Some((child_size, count)) = split_spec(size, self.chaos_mode) else { return };
        for i in 0..count {
            let seed = self.next_seed();
            let child_vel = split_velocity(vel, i, count, seed);
            let child_pos = pos + split_offset(vel, size, child_size, i, count);
            let spin = (hash_f32(seed.wrapping_add(7)) - 0.5) * 2.0 * ASTEROID_SPIN_MAX;
            self.spawn_asteroid(world, child_size, child_pos, child_vel, spin, child_machine());
        }
    }

    /// Teleport anything that left the field to the opposite side, once its farthest
    /// drawn pixel has. Writing `Transform2D.position` on a live dynamic body is a rapier
    /// teleport that preserves velocity — proven by a sim test.
    pub(crate) fn wrap_entities(&mut self, ctx: &mut GameContext) {
        let half = Vec2::new(WIN_W / 2.0, WIN_H / 2.0);
        let mut targets: Vec<(EntityId, f32)> = Vec::new();
        for ship in &self.ships {
            if let Some(entity) = ship.entity {
                targets.push((entity, FUNK_REACH));
            }
            targets.extend(ship.bullets.iter().map(|&b| (b, SHOT_REACH)));
        }
        targets.extend(self.asteroids.iter().map(|r| (r.entity, MEATIEROID_REACH[r.size.idx()])));

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
                if self.total_lives() == 1 {
                    ctx.achievements.unlock(achievements::LIVING_ON_EDGE);
                }
            }
            self.spawn_wave(ctx.world);
        }
    }
}
