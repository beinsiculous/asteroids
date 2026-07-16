//! Ship control law, firing, and death/respawn. The whole flight model is
//! the pure `integrate_ship` — gameplay reads rapier's velocity + heading,
//! integrates, and pushes the result back with one `set_velocity` per frame.

use engine_core::prelude::*;
use std::f32::consts::FRAC_PI_2;

use super::entity_position;
use super::rocks::circles_overlap;
use crate::constants::*;
use crate::effects;
use crate::types::*;

/// One frame of the flight model. `turn` is -1/0/+1 (+1 = counterclockwise),
/// `rot` is the current heading. Thrust accelerates along the heading, drag
/// bleeds speed every frame, and the result is clamped to `SHIP_MAX_SPEED`.
pub(crate) fn integrate_ship(vel: Vec2, rot: f32, turn: f32, thrust: bool, dt: f32) -> (Vec2, f32) {
    let mut vel = vel;
    if thrust {
        vel += Vec2::new(rot.cos(), rot.sin()) * SHIP_THRUST * dt;
    }
    vel *= (1.0 - SHIP_DRAG * dt).max(0.0);
    if vel.length() > SHIP_MAX_SPEED {
        vel = vel.normalize() * SHIP_MAX_SPEED;
    }
    (vel, turn * SHIP_TURN_SPEED)
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
    /// Read input for every live ship, run the flight model, push one
    /// `set_velocity` each, and fire. In co-op each ship reads its own
    /// player; in single player the lone ship reads both players merged so
    /// either keyboard cluster or pad flies it.
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
            self.ships[index].thrusting = thrust;

            let (vel, _) = self.physics.get_body_velocity(entity).unwrap_or((Vec2::ZERO, 0.0));
            let rot =
                ctx.world.get::<Transform2D>(entity).map(|t| t.rotation).unwrap_or(FRAC_PI_2);
            let (new_vel, angular) = integrate_ship(vel, rot, turn, thrust, ctx.delta_time);
            self.physics.set_velocity(entity, new_vel, angular);

            self.update_firing(ctx, index, fire);
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
        let Some((pos, forward)) =
            ctx.world.get::<Transform2D>(entity).map(|t| (t.position, t.forward()))
        else {
            return;
        };
        self.spawn_bullet(ctx.world, index, pos + forward * MUZZLE_OFFSET, forward * BULLET_SPEED);
        self.ships[index].fire_cooldown = FIRE_COOLDOWN;
    }

    /// Ship-vs-rock is a game-side circle test, not a collision event: a
    /// rock that drifted over a ship during i-frames must still kill the
    /// moment mercy runs out, and a stale `started` event can't do that.
    /// Each live ship is checked against every rock with its own i-frames.
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
                    circles_overlap(
                        ship_pos,
                        SHIP_COLLIDER_RADIUS,
                        p,
                        rock.size.radius() * ASTEROID_COLLIDER_SCALE,
                    )
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

    /// Destroy ship `index`: burst, grid ripple, dock a life. At zero lives
    /// the body is despawned and the survivor plays on; the match only ends
    /// once every ship is out (`coop_over`).
    fn kill_ship(&mut self, ctx: &mut GameContext, index: usize) {
        let Some(entity) = self.ships[index].entity else { return };
        let pos = entity_position(ctx.world, entity).unwrap_or(Vec2::ZERO);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::ship_death_burst(&theme, self.tex_id));
        let (strength, radius) = GRID_IMPULSE_SHIP_DEATH;
        self.ripple_grid(pos, strength, radius);

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

    /// Park ship `index` at its spawn pose with zero velocity. Writing
    /// `Transform2D` directly teleports the live rapier body (GPP-09) — and
    /// doubles as the initial pose before the body first syncs.
    pub(crate) fn reset_ship_pose(&mut self, world: &mut World, index: usize) {
        let Some(entity) = self.ships[index].entity else { return };
        let (pos, facing) = crate::spawning::ship_spawn_pose(self.ships.len(), index);
        if let Some(t) = world.get_mut::<Transform2D>(entity) {
            t.position = pos;
            t.rotation = facing;
        }
        self.physics.set_velocity(entity, Vec2::ZERO, 0.0);
    }
}
