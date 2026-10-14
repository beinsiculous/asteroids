//! Per-frame gameplay orchestration. The pieces live in submodules: ship
//! control, its drawn pose and death (`ship`), meatieroid splitting/waves/wrapping
//! (`rocks`), and match lifecycle (`flow`).

mod flow;
mod rocks;
mod ship;

pub(crate) use rocks::{asteroid_speed, spawn_position, wave_asteroid_count};
pub(crate) use ship::blink_visible;

#[cfg(test)]
pub(crate) use rocks::{
    circles_overlap, resolve_hit_pairs, split_offset, split_spec, split_velocity, wrap_position,
};
#[cfg(test)]
pub(crate) use ship::{
    advance_heading, can_fire, coop_over, displayed_heading, heading_direction, integrate_ship,
    invincibility_secs, ship_pose,
};

use engine_core::prelude::*;

use crate::constants::*;
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
        // top so any sprite-vs-collider mismatch is obvious.
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        // Pause gate: only an active match is pausable. While paused the whole
        // update is frozen — no physics step, no ship control, no timers; the
        // overlay draws in the UI pass, and the engine holds the sprites and the
        // grid still.
        if self.state == GameState::Playing {
            let action = self.pause.update(ctx.players, ctx.input, ctx.window_size);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
                PauseAction::ExitGame => { ctx.request_exit(); return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                self.emit_collider_overlay(ctx);
                return;
            }
        }

        self.update_ship_control(ctx);
        self.physics.update(ctx.world, ctx.delta_time);
        self.prune_expired_bullets(ctx);
        self.prune_ended_one_shots(ctx.world);

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
        // Wrap in GameOver too, so the leftover meatieroids keep circling the
        // field behind the overlay instead of drifting off forever.
        self.wrap_entities(ctx);
        self.pose_ships(ctx.world);
        self.emit_collider_overlay(ctx);
    }

    /// Outline every collider in bright magenta while F1 is on — the only line-buffer
    /// work the game owns, drawn over the art.
    fn emit_collider_overlay(&self, ctx: &mut GameContext) {
        if self.debug_colliders {
            debug::draw_colliders(ctx.world, ctx.lines, DEBUG_COLLIDER_COLOR, DEBUG_COLLIDER_EMISSIVE);
        }
    }

    /// Bullets the engine's lifetime pass (`ecs::LifetimeSystem`, stepped by
    /// the frame tail) despawned this frame expired without hitting anything —
    /// that's a miss, so the owning ship's sharpshooter streak resets.
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

    /// Forget every `die` and `crash` whose clip ran out: the engine despawned it, and the
    /// list would otherwise grow with every kill of an endless match.
    fn prune_ended_one_shots(&mut self, world: &World) {
        let alive = world.entities();
        self.transient_visuals.retain(|one_shot| alive.contains(one_shot));
    }

    /// The match's sprites show only in a match (playing or game over); the stars and
    /// the grid stay up under the menus. A ship blinks through its i-frames.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let in_match = matches!(self.state, GameState::Playing | GameState::GameOver);
        let entities: Vec<EntityId> = self.asteroids.iter().map(|rock| rock.entity)
            .chain(self.ships.iter().flat_map(|ship| ship.bullets.iter().copied()))
            .chain(self.transient_visuals.iter().copied())
            .collect();
        set_sprites_visible(ctx.world, entities, in_match);

        for ship in &self.ships {
            if let Some(entity) = ship.entity {
                set_sprites_visible(ctx.world, [entity], in_match && blink_visible(ship.invincibility));
            }
        }
    }
}

/// Queue a radial shockwave for the deforming grid; the engine applies it next frame.
pub(crate) fn ripple_grid(world: &mut World, position: Vec2, strength: f32, radius: f32) {
    ripple(world, GridImpulse::Radial { position, strength, radius, attractive: false });
}
