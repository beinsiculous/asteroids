//! Per-frame gameplay orchestration. The pieces live in submodules: ship
//! control and death (`ship`), rock splitting/waves/wrapping (`rocks`),
//! and match lifecycle (`flow`).

mod flow;
mod rocks;
mod ship;

pub(crate) use rocks::{asteroid_speed, spawn_position, wave_asteroid_count};
pub(crate) use ship::blink_visible;

#[cfg(test)]
pub(crate) use rocks::{circles_overlap, resolve_hit_pairs, split_spec, split_velocity, wrap_position};
#[cfg(test)]
pub(crate) use ship::{can_fire, coop_over, integrate_ship, invincibility_secs};

use engine_core::prelude::*;

use crate::types::*;

pub(crate) fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

/// World positions of every ship that still has a live body — the set new
/// rocks keep clear of, and the set ship-vs-rock hit checks iterate.
pub(crate) fn live_ship_positions(ships: &[ShipState], world: &World) -> Vec<Vec2> {
    ships
        .iter()
        .filter_map(|s| s.entity)
        .filter_map(|e| entity_position(world, e))
        .collect()
}

impl AsteroidsGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // F1 toggles the collider debug overlay. Magenta outlines render on
        // top so any wireframe-vs-collider mismatch is obvious.
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        // Pause gate: only an active match is pausable. While paused the whole
        // update is frozen — no physics step, no ship control, no timers; the
        // overlay draws in the UI pass. The wireframes are re-emitted so the
        // frozen scene stays visible beneath the overlay (the update that
        // normally pushes them is skipped).
        if self.state == GameState::Playing {
            let action = self.pause.update(ctx.players, ctx.input);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
                PauseAction::ExitGame => { ctx.exit_requested = true; return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                // Keep the frozen scene visible under the pause overlay:
                // wireframes re-emitted, grid re-emitted without advancing.
                self.emit_wireframes(ctx);
                engine_core::grid::step_and_emit_grid(
                    self.grid.as_mut(), ctx.world, ctx.lines, 0.0, self.debug_colliders,
                );
                return;
            }
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
            for ship in &mut self.ships {
                ship.invincibility = (ship.invincibility - ctx.delta_time).max(0.0);
            }
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
    /// hitting anything — that's a miss, so the owning ship's sharpshooter
    /// streak resets.
    fn prune_expired_bullets(&mut self, ctx: &mut GameContext) {
        let alive = ctx.world.entities();
        for ship in &mut self.ships {
            let before = ship.bullets.len();
            ship.bullets.retain(|b| alive.contains(b));
            if ship.bullets.len() < before {
                ship.shot_streak = 0;
            }
        }
    }

    /// Push a radial shockwave into the deforming grid.
    pub(crate) fn ripple_grid(&mut self, position: Vec2, strength: f32, radius: f32) {
        if let Some(grid) = self.grid.as_mut() {
            grid.apply_impulse(&GridImpulse::Radial { position, strength, radius, attractive: false });
        }
    }

    /// The only gameplay sprites are bullets (ships and rocks are wireframes)
    /// — hide them on the menu screens for convention parity.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = matches!(self.state, GameState::Playing | GameState::GameOver);
        let bullets: Vec<EntityId> =
            self.ships.iter().flat_map(|s| s.bullets.iter().copied()).collect();
        set_sprites_visible(ctx.world, bullets, visible);
    }
}
