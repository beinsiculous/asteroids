//! Per-frame gameplay orchestration. The pieces live in submodules: ship
//! control and death (`ship`), rock splitting/waves/wrapping (`rocks`),
//! and match lifecycle (`flow`).

mod flow;
mod rocks;
mod ship;

pub(crate) use rocks::{asteroid_speed, spawn_position, wave_asteroid_count};
pub(crate) use ship::blink_visible;

#[cfg(test)]
pub(crate) use rocks::{circles_overlap, split_spec, split_velocity, wrap_position};
#[cfg(test)]
pub(crate) use ship::{can_fire, integrate_ship, invincibility_secs};

use engine_core::prelude::*;

use crate::types::*;

pub(crate) fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

impl AsteroidsGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // F1 toggles the collider debug overlay. Magenta outlines render on
        // top so any wireframe-vs-collider mismatch is obvious.
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        self.update_ship_control(ctx);
        self.physics.update(ctx.world, ctx.delta_time);
        // Expired bullets despawn here; the physics system garbage-collects
        // their rapier bodies on its next update.
        self.lifetimes.update(ctx.world, ctx.delta_time);
        self.prune_expired_bullets(ctx);

        // Drain this frame's collision events once (take = the buffer is
        // consumed, not borrowed). Every consumer below shares this Vec.
        let collisions: Vec<CollisionData> = self.physics.take_collision_events();

        self.handle_state_input(ctx);
        if self.state == GameState::Playing {
            self.play_time += ctx.delta_time;
            self.invincibility = (self.invincibility - ctx.delta_time).max(0.0);
            self.resolve_bullet_hits(ctx, &collisions);
            self.check_ship_hit(ctx);
            self.check_wave_clear(ctx);
        }
        // Wrap in GameOver too, so the leftover rocks keep circling the
        // field behind the overlay instead of drifting off forever.
        self.wrap_entities(ctx);

        // Step + render the deforming grid after gameplay so it reacts to
        // this frame's events; the wireframes go on top of the grid.
        step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
        self.emit_wireframes(ctx);
    }

    /// Bullets the `LifetimeSystem` despawned this frame expired without
    /// hitting anything — that's a miss, so the sharpshooter streak resets.
    fn prune_expired_bullets(&mut self, ctx: &mut GameContext) {
        let before = self.bullets.len();
        let alive = ctx.world.entities();
        self.bullets.retain(|b| alive.contains(b));
        if self.bullets.len() < before {
            self.shot_streak = 0;
        }
    }

    /// Push a radial shockwave into the deforming grid.
    pub(crate) fn ripple_grid(&mut self, position: Vec2, strength: f32, radius: f32) {
        if let Some(grid) = self.grid.as_mut() {
            grid.apply_impulse(&GridImpulse::Radial { position, strength, radius, attractive: false });
        }
    }

    /// The only gameplay sprites are bullets (ship and rocks are wireframes)
    /// — hide them on the menu screens for convention parity.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = matches!(self.state, GameState::Playing | GameState::GameOver);
        set_sprites_visible(ctx.world, self.bullets.clone(), visible);
    }
}
