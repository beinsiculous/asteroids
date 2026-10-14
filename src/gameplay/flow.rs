//! Match lifecycle: state-machine input during play, starting/ending a
//! match, scoring, and pushing the chaos theme onto live entities.

use engine_core::prelude::*;

use crate::art::Pilot;
use crate::constants::*;
use crate::spawning::{ship_spawn_pose, spawn_ship};
use crate::types::*;

impl AsteroidsGame {
    /// State changes while the simulation screens are up, driven through the
    /// per-player facade. During play, Menu (Escape / pad Start) is handled by
    /// the pause gate, not here. From game over, Action1 (Space / Enter / pad
    /// A) restarts and Menu bails to the title.
    pub(crate) fn handle_state_input(&mut self, ctx: &mut GameContext) {
        let primary = ctx.players.just_activated_any(GameAction::Action1, ctx.input);
        let menu = ctx.players.just_activated_any(GameAction::Menu, ctx.input);
        if let GameState::GameOver = &self.state {
            if primary {
                self.start_game(ctx);
            } else if menu {
                self.reset_to_title(ctx.world);
            }
        }
    }

    /// Reset every counter, clear the field, spawn the ships for the current
    /// mode, and start wave 1. Called from mode select and game-over restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.wave = 1;
        self.wave_clear_timer = 0.0;
        self.waves_without_death = 0;
        self.last_kill_time = f32::NEG_INFINITY;
        self.play_time = 0.0;

        self.destroy_all_bullets(ctx.world);
        self.destroy_all_asteroids(ctx.world);
        self.clear_transient_visuals(ctx.world);
        self.spawn_ships(ctx.world);
        self.apply_theme(ctx.world);
        self.spawn_wave(ctx.world);
        self.state = GameState::Playing;
    }

    /// Build a fresh set of ships for the current mode, despawning any that
    /// linger from a previous match. Each gets its pilot's Funk, its own spawn
    /// point and heading, and a full count of lives.
    pub(crate) fn spawn_ships(&mut self, world: &mut World) {
        let stale: Vec<EntityId> =
            self.ships.iter_mut().filter_map(|s| s.entity.take()).collect();
        for entity in stale {
            self.physics.destroy_entity(world, entity);
        }
        let count = self.mode.player_count();
        self.ships = (0..count)
            .map(|index| {
                let (pos, facing) = ship_spawn_pose(count, index);
                let entity = spawn_ship(world, &self.sheets, Pilot::for_ship(index), pos);
                ShipState { heading: facing, ..ShipState::new(entity) }
            })
            .collect();
    }

    /// Total lives remaining across every ship — drives the shared
    /// "last ship" achievement and reads as one ship's lives in solo.
    pub(crate) fn total_lives(&self) -> u32 {
        self.ships.iter().map(|s| s.lives).sum()
    }

    /// Out of ships. The meatieroids keep drifting behind the game-over overlay;
    /// the next start clears them. This is the only transition into
    /// GameOver, so final scores are submitted here exactly once per match:
    /// the lone ship under "single", each co-op ship under "coop".
    pub(crate) fn finish_game(&mut self, ctx: &mut GameContext) {
        self.destroy_all_bullets(ctx.world);
        let mode_key = match self.mode {
            GameMode::SinglePlayer => "single",
            GameMode::TwoPlayerCoop => "coop",
        };
        for ship in &mut self.ships {
            ship.thrusting = false;
            let _ = ctx.scores.submit(mode_key, u64::from(ship.score));
        }
        self.state = GameState::GameOver;
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_all_bullets(world);
        self.destroy_all_asteroids(world);
        self.clear_transient_visuals(world);
        let ships: Vec<EntityId> =
            self.ships.iter_mut().filter_map(|s| s.entity.take()).collect();
        for entity in ships {
            self.physics.destroy_entity(world, entity);
        }
        self.ships.clear();
        self.state = GameState::TitleScreen { selection: 0 };
    }

    pub(crate) fn destroy_all_bullets(&mut self, world: &mut World) {
        let bullets: Vec<EntityId> =
            self.ships.iter_mut().flat_map(|s| s.bullets.drain(..)).collect();
        for bullet in bullets {
            self.physics.destroy_entity(world, bullet);
        }
    }

    pub(crate) fn destroy_all_asteroids(&mut self, world: &mut World) {
        for rock in std::mem::take(&mut self.asteroids) {
            self.physics.destroy_entity(world, rock.entity);
        }
    }

    /// End every `die` and `crash` still playing. An id the world has already dropped —
    /// a one-shot whose clip ran out — is skipped: ids are generational, so a stale entry
    /// can never name a newer entity.
    pub(crate) fn clear_transient_visuals(&mut self, world: &mut World) {
        for entity in self.transient_visuals.drain(..) {
            world.remove_entity(&entity).ok();
        }
    }

    /// Add points to ship `ship_index`, unlock score tiers, and grant that
    /// ship a bonus life at every `EXTRA_LIFE_EVERY` crossing (each threshold
    /// pays out exactly once, per ship). A ship already out of lives still scores
    /// the shots it left in flight but never earns a life back: one it has no body
    /// to fly would keep `coop_over` from ever ending the match.
    pub(crate) fn award_points(
        &mut self,
        achievements: &mut AchievementManager,
        ship_index: usize,
        points: u32,
    ) {
        let ship = &mut self.ships[ship_index];
        ship.score += points;
        if ship.score >= SCORE_TIER_1 {
            achievements.unlock(crate::achievements::SCORE_10K);
        }
        if ship.score >= SCORE_TIER_2 {
            achievements.unlock(crate::achievements::SCORE_30K);
        }
        while ship.lives > 0 && ship.score >= ship.next_extra_life_at {
            ship.lives += 1;
            ship.next_extra_life_at += EXTRA_LIFE_EVERY;
        }
    }

    /// Push the current `chaos_mode`'s look onto the grid. The mode pick happens after
    /// `init()` spawned it, so the colour it was born with is the boot mode's.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let color = ChaosTheme::for_mode(self.chaos_mode).grid_color;
        if let Some(backdrop) = self.backdrop {
            if let Some(grid) = world.get_mut::<GridBackdrop>(backdrop) {
                grid.color = color;
            }
        }
    }
}
