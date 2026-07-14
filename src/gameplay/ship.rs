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

impl AsteroidsGame {
    /// Read input, run the flight model, push one `set_velocity`, and fire.
    pub(crate) fn update_ship_control(&mut self, ctx: &mut GameContext) {
        if self.state != GameState::Playing {
            self.thrusting = false;
            return;
        }
        let Some(ship) = self.ship else { return };

        let left = ctx.input.is_key_pressed(KeyCode::ArrowLeft)
            || ctx.input.is_key_pressed(KeyCode::KeyA);
        let right = ctx.input.is_key_pressed(KeyCode::ArrowRight)
            || ctx.input.is_key_pressed(KeyCode::KeyD);
        let turn = match (left, right) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        };
        let thrust = ctx.input.is_key_pressed(KeyCode::ArrowUp)
            || ctx.input.is_key_pressed(KeyCode::KeyW);
        self.thrusting = thrust;

        let (vel, _) = self.physics.get_body_velocity(ship).unwrap_or((Vec2::ZERO, 0.0));
        let rot = ctx.world.get::<Transform2D>(ship).map(|t| t.rotation).unwrap_or(FRAC_PI_2);
        let (new_vel, angular) = integrate_ship(vel, rot, turn, thrust, ctx.delta_time);
        self.physics.set_velocity(ship, new_vel, angular);

        self.update_firing(ctx);
    }

    fn update_firing(&mut self, ctx: &mut GameContext) {
        self.fire_cooldown = (self.fire_cooldown - ctx.delta_time).max(0.0);
        if !ctx.input.is_key_just_pressed(KeyCode::Space) {
            return;
        }
        if !can_fire(self.fire_cooldown, self.bullets.len()) {
            return;
        }
        let Some(ship) = self.ship else { return };
        let Some((pos, forward)) =
            ctx.world.get::<Transform2D>(ship).map(|t| (t.position, t.forward()))
        else {
            return;
        };
        self.spawn_bullet(ctx.world, pos + forward * MUZZLE_OFFSET, forward * BULLET_SPEED);
        self.fire_cooldown = FIRE_COOLDOWN;
    }

    /// Ship-vs-rock is a game-side circle test, not a collision event: a
    /// rock that drifted over the ship during i-frames must still kill the
    /// moment mercy runs out, and a stale `started` event can't do that.
    pub(crate) fn check_ship_hit(&mut self, ctx: &mut GameContext) {
        if self.invincibility > 0.0 {
            return;
        }
        let Some(ship) = self.ship else { return };
        let Some(ship_pos) = entity_position(ctx.world, ship) else { return };
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
            self.kill_ship(ctx);
        }
    }

    fn kill_ship(&mut self, ctx: &mut GameContext) {
        let Some(ship) = self.ship else { return };
        let pos = entity_position(ctx.world, ship).unwrap_or(Vec2::ZERO);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::ship_death_burst(&theme, self.tex_id));
        let (strength, radius) = GRID_IMPULSE_SHIP_DEATH;
        self.ripple_grid(pos, strength, radius);

        self.lives = self.lives.saturating_sub(1);
        self.waves_without_death = 0;
        self.shot_streak = 0;

        if self.lives == 0 {
            self.finish_game(ctx);
        } else {
            self.reset_ship_pose(ctx.world);
            self.invincibility = invincibility_secs(self.chaos_mode);
        }
    }

    /// Park the ship at the center facing up with zero velocity. Writing
    /// `Transform2D` directly teleports the live rapier body (GPP-09) — and
    /// doubles as the initial pose before the body first syncs.
    pub(crate) fn reset_ship_pose(&mut self, world: &mut World) {
        let Some(ship) = self.ship else { return };
        if let Some(t) = world.get_mut::<Transform2D>(ship) {
            t.position = Vec2::ZERO;
            t.rotation = FRAC_PI_2;
        }
        self.physics.set_velocity(ship, Vec2::ZERO, 0.0);
    }
}
