//! Ship control law, the drawn pose, firing, and death/respawn.
//!
//! The body's rotation is locked; the heading lives in `ShipState.heading`, and the
//! sprite shows the nearest of the sixteen drawn headings. Thrust and fire both follow
//! that drawn heading, so the ship goes and shoots where it is seen to point. The flight
//! model is the pure `integrate_ship`: gameplay reads rapier's velocity, integrates, and
//! pushes the result back with one `set_velocity` per frame.

use engine_core::prelude::*;
use std::f32::consts::TAU;

use super::entity_position;
use super::ripple_grid;
use super::rocks::circles_overlap;
use crate::art::*;
use crate::constants::*;
use crate::effects;
use crate::music::Moment;
use crate::spawning::{spawn_effect, Effect};
use crate::types::*;

/// The angle between adjacent drawn headings.
const HEADING_STEP: f32 = TAU / HEADINGS as f32;

/// One frame of turning: `turn` is -1/0/+1 (+1 = counterclockwise). The result stays in
/// [0, 2π), so a right turn from east wraps to just under 2π instead of going negative.
pub(crate) fn advance_heading(heading: f32, turn: f32, dt: f32) -> f32 {
    let wrapped = (heading + turn * SHIP_TURN_SPEED * dt).rem_euclid(TAU);
    // `rem_euclid` can round a tiny negative up to exactly 2π.
    if wrapped >= TAU { 0.0 } else { wrapped }
}

/// The drawn heading nearest `heading`, 0..16 and never negative.
pub(crate) fn displayed_heading(heading: f32) -> usize {
    ((heading / HEADING_STEP).round() as i64).rem_euclid(HEADINGS as i64) as usize
}

/// The direction of the drawn heading nearest `heading`: where the ship thrusts and fires.
pub(crate) fn heading_direction(heading: f32) -> Vec2 {
    let angle = displayed_heading(heading) as f32 * HEADING_STEP;
    Vec2::new(angle.cos(), angle.sin())
}

/// One frame of the flight model. Thrust accelerates along `direction`, drag bleeds
/// speed every frame, and the result is clamped to `SHIP_MAX_SPEED`.
pub(crate) fn integrate_ship(vel: Vec2, direction: Vec2, thrust: bool, dt: f32) -> Vec2 {
    let mut vel = vel;
    if thrust {
        vel += direction * SHIP_THRUST * dt;
    }
    vel *= (1.0 - SHIP_DRAG * dt).max(0.0);
    if vel.length() > SHIP_MAX_SPEED {
        vel = vel.normalize() * SHIP_MAX_SPEED;
    }
    vel
}

/// A drawn pose: which clip, and which frame of that clip's own frame list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShipPose {
    pub(crate) clip: &'static str,
    pub(crate) frame: usize,
}

/// What a ship heading `heading`, thrusting or not, shows `play_time` seconds into the
/// match. The frame comes from the match clock rather than from the clip's own start,
/// so turning through a heading lands mid-blink on the right frame instead of
/// restarting the clip at every sixteenth of a turn.
pub(crate) fn ship_pose(heading: f32, thrusting: bool, play_time: f32) -> ShipPose {
    let index = displayed_heading(heading);
    let (clips, frame_seconds) = if thrusting {
        (&THRUST_HEADINGS, THRUST_FRAME_SECONDS)
    } else {
        (&IDLE_HEADINGS, IDLE_FRAME_SECONDS)
    };
    let frame = (play_time.max(0.0) / frame_seconds) as usize % HEADING_CLIP_FRAMES;
    ShipPose { clip: clips[index], frame }
}

/// Write a pose onto a ship: the clip and the frame, held still. `play` is deliberately
/// not used — it restarts the clip.
fn write_pose(world: &mut World, entity: EntityId, pose: ShipPose) {
    let Some(animation) = world.get_mut::<SpriteAnimation>(entity) else {
        return;
    };
    animation.current_clip = Some(pose.clip.to_string());
    animation.current_frame = pose.frame;
    animation.playing = false;
    animation.finished = false;
}

pub(crate) fn can_fire(cooldown: f32, live_bullets: usize) -> bool {
    cooldown <= 0.0 && live_bullets < MAX_LIVE_BULLETS
}

/// Post-respawn i-frame duration; the Insane family cuts mercy in half.
pub(crate) fn invincibility_secs(mode: ChaosMode) -> f32 {
    let mult = if mode.is_insane() { INSANE_INVINCIBILITY_MULT } else { 1.0 };
    RESPAWN_INVINCIBILITY * mult
}

/// Whether the ship is drawn this frame: always when vulnerable, blinking
/// at `INVINCIBILITY_BLINK_HZ` during i-frames.
pub(crate) fn blink_visible(invincibility: f32) -> bool {
    invincibility <= 0.0 || (invincibility * INVINCIBILITY_BLINK_HZ) as i32 % 2 == 0
}

/// The co-op match is over only once every ship is out of lives; a single
/// survivor keeps the game going. In single player this is just "lives == 0".
pub(crate) fn coop_over(lives: &[u32]) -> bool {
    lives.iter().all(|&l| l == 0)
}

impl AsteroidsGame {
    /// Read input for every live ship, turn its heading, run the flight model, push one
    /// `set_velocity` each, and fire. In co-op each ship reads its own player; in single
    /// player the lone ship reads both players merged so either keyboard cluster or pad
    /// flies it.
    pub(crate) fn update_ship_control(&mut self, ctx: &mut GameContext) {
        if self.state != GameState::Playing {
            for ship in &mut self.ships {
                ship.thrusting = false;
            }
            return;
        }
        for index in 0..self.ships.len() {
            let Some(entity) = self.ships[index].entity else {
                self.ships[index].thrusting = false;
                continue;
            };
            let (turn, thrust, fire) = self.ship_input(ctx, index);
            let ship = &mut self.ships[index];
            let thrust_started = thrust && !ship.thrusting;
            ship.thrusting = thrust;
            ship.heading = advance_heading(ship.heading, turn, ctx.delta_time);
            let direction = heading_direction(ship.heading);

            let (vel, _) = self.physics.get_body_velocity(entity).unwrap_or((Vec2::ZERO, 0.0));
            let new_vel = integrate_ship(vel, direction, thrust, ctx.delta_time);
            self.physics.set_velocity(entity, new_vel, 0.0);
            if thrust_started {
                self.music.play(ctx, Moment::Thrust(entity));
            }

            self.update_firing(ctx, index, fire);
        }
    }

    /// Write every live ship's drawn pose for this frame.
    pub(crate) fn pose_ships(&self, world: &mut World) {
        for ship in &self.ships {
            if let Some(entity) = ship.entity {
                write_pose(world, entity, ship_pose(ship.heading, ship.thrusting, self.play_time));
            }
        }
    }

    /// Flight controls for ship `index` this frame: `(turn, thrust, fire)`.
    /// Turn is `-move_x` (bank the ship toward the pressed direction); thrust
    /// is up on the stick or MoveUp; fire is a fresh Action1. Single player
    /// sums both players' movement (clamped) and takes either one's fire.
    fn ship_input(&self, ctx: &GameContext, index: usize) -> (f32, bool, bool) {
        match self.mode {
            GameMode::SinglePlayer => {
                let move_x = (ctx.players.move_x(PlayerId::P1, ctx.input)
                    + ctx.players.move_x(PlayerId::P2, ctx.input))
                .clamp(-1.0, 1.0);
                let thrust = ctx.players.move_y(PlayerId::P1, ctx.input) > 0.0
                    || ctx.players.move_y(PlayerId::P2, ctx.input) > 0.0
                    || ctx.players.is_active(PlayerId::P1, GameAction::MoveUp, ctx.input)
                    || ctx.players.is_active(PlayerId::P2, GameAction::MoveUp, ctx.input);
                let fire = ctx.players.just_activated(PlayerId::P1, GameAction::Action1, ctx.input)
                    || ctx.players.just_activated(PlayerId::P2, GameAction::Action1, ctx.input);
                (-move_x, thrust, fire)
            }
            GameMode::TwoPlayerCoop => {
                let player = PlayerId(index as u8);
                let move_x = ctx.players.move_x(player, ctx.input);
                let thrust = ctx.players.move_y(player, ctx.input) > 0.0
                    || ctx.players.is_active(player, GameAction::MoveUp, ctx.input);
                let fire = ctx.players.just_activated(player, GameAction::Action1, ctx.input);
                (-move_x, thrust, fire)
            }
        }
    }

    fn update_firing(&mut self, ctx: &mut GameContext, index: usize, fire: bool) {
        self.ships[index].fire_cooldown =
            (self.ships[index].fire_cooldown - ctx.delta_time).max(0.0);
        if !fire {
            return;
        }
        let ship = &self.ships[index];
        if !can_fire(ship.fire_cooldown, ship.bullets.len()) {
            return;
        }
        let Some(entity) = ship.entity else { return };
        let Some(pos) = entity_position(ctx.world, entity) else { return };
        let direction = heading_direction(ship.heading);
        self.spawn_bullet(ctx.world, index, pos + direction * MUZZLE_OFFSET, direction * BULLET_SPEED);
        self.ships[index].fire_cooldown = FIRE_COOLDOWN;
        self.music.play(ctx, Moment::Shot(entity));
    }

    /// Ship-vs-meatieroid is a game-side circle test, not a collision event: a
    /// meatieroid that drifted over a ship during i-frames must still kill the
    /// moment mercy runs out, and a stale `started` event can't do that.
    /// Each live ship is checked against every body with its own i-frames.
    pub(crate) fn check_ship_hit(&mut self, ctx: &mut GameContext) {
        let mut killed: Vec<usize> = Vec::new();
        for index in 0..self.ships.len() {
            let ship = &self.ships[index];
            if ship.invincibility > 0.0 {
                continue;
            }
            let Some(entity) = ship.entity else { continue };
            let Some(ship_pos) = entity_position(ctx.world, entity) else { continue };
            let hit = self.asteroids.iter().any(|rock| {
                entity_position(ctx.world, rock.entity).is_some_and(|p| {
                    circles_overlap(ship_pos, SHIP_COLLIDER_RADIUS, p, rock.size.radius())
                })
            });
            if hit {
                killed.push(index);
            }
        }
        for index in killed {
            self.kill_ship(ctx, index);
        }
    }

    /// Destroy ship `index`: its crash plays where it was, turned to its drawn heading,
    /// with a burst and a grid ripple, and it docks a life. At zero lives the body is
    /// despawned and the survivor plays on; the match only ends once every ship is out
    /// (`coop_over`).
    fn kill_ship(&mut self, ctx: &mut GameContext, index: usize) {
        let Some(entity) = self.ships[index].entity else { return };
        let pos = entity_position(ctx.world, entity).unwrap_or(Vec2::ZERO);
        let pilot = Pilot::for_ship(index);
        let art = pilot.art();
        let crash = spawn_effect(ctx.world, &Effect {
            name: "Crash",
            spec: art.sheet,
            sheet: self.sheets.funk(pilot),
            depth: art.depth,
            clip: CRASH,
            position: pos,
            rotation: displayed_heading(self.ships[index].heading) as f32 * HEADING_STEP,
        });
        self.transient_visuals.push(crash);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::ship_death_burst(art.burst_color, &theme, self.tex_id));
        let (strength, radius) = GRID_IMPULSE_SHIP_DEATH;
        ripple_grid(ctx.world, pos, strength, radius);
        self.music.play(ctx, Moment::Crash);

        self.ships[index].lives = self.ships[index].lives.saturating_sub(1);
        self.ships[index].shot_streak = 0;
        self.waves_without_death = 0;

        if self.ships[index].lives == 0 {
            self.physics.destroy_entity(ctx.world, entity);
            self.ships[index].entity = None;
            let lives: Vec<u32> = self.ships.iter().map(|s| s.lives).collect();
            if coop_over(&lives) {
                self.finish_game(ctx);
            }
        } else {
            self.reset_ship_pose(ctx.world, index);
            self.ships[index].invincibility = invincibility_secs(self.chaos_mode);
        }
    }

    /// Park ship `index` at its spawn point, facing its spawn heading, with zero
    /// velocity. Writing `Transform2D.position` teleports the live rapier body;
    /// the rotation is left at 0, because the heading turns the drawn ship, not the body.
    pub(crate) fn reset_ship_pose(&mut self, world: &mut World, index: usize) {
        let Some(entity) = self.ships[index].entity else { return };
        let (pos, facing) = crate::spawning::ship_spawn_pose(self.ships.len(), index);
        self.ships[index].heading = facing;
        if let Some(t) = world.get_mut::<Transform2D>(entity) {
            t.position = pos;
        }
        self.physics.set_velocity(entity, Vec2::ZERO, 0.0);
    }
}
