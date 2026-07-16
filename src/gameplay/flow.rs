//! Match lifecycle: state-machine input during play, starting/ending a
//! match, scoring, and pushing the chaos theme onto live entities.

use engine_core::prelude::*;

use crate::constants::*;
use crate::spawning::ship_spawn_pose;
use crate::types::*;

impl AsteroidsGame {
    /// State changes while the simulation screens are up, driven through the
    /// per-player facade: either player's Menu action (Escape / pad Start)
    /// bails to the title, and Action1 (Space / Enter / pad A) restarts from
    /// game over.
    pub(crate) fn handle_state_input(&mut self, ctx: &mut GameContext) {
        let primary = ctx.players.just_activated_any(GameAction::Action1, ctx.input);
        let menu = ctx.players.just_activated_any(GameAction::Menu, ctx.input);
        match &self.state {
            GameState::Playing => {
                if menu {
                    self.reset_to_title(ctx.world);
                }
            }
            GameState::GameOver => {
                if primary {
                    self.start_game(ctx);
                } else if menu {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
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
        self.spawn_ships(ctx.world);
        self.apply_theme(ctx.world);
        self.spawn_wave(ctx.world);
        self.state = GameState::Playing;
    }

    /// Build a fresh set of ships for the current mode, despawning any that
    /// linger from a previous match. Each gets its own spawn pose and a full
    /// count of lives.
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
                ShipState::new(crate::spawning::spawn_ship(world, pos, facing))
            })
            .collect();
    }

    /// Total lives remaining across every ship — drives the shared
    /// "last ship" achievement and reads as one ship's lives in solo.
    pub(crate) fn total_lives(&self) -> u32 {
        self.ships.iter().map(|s| s.lives).sum()
    }

    /// Out of ships. The rocks keep drifting behind the game-over overlay;
    /// the next start clears them.
    pub(crate) fn finish_game(&mut self, ctx: &mut GameContext) {
        self.destroy_all_bullets(ctx.world);
        for ship in &mut self.ships {
            ship.thrusting = false;
        }
        self.state = GameState::GameOver;
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_all_bullets(world);
        self.destroy_all_asteroids(world);
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
        for rock in self.asteroids.drain(..).collect::<Vec<_>>() {
            self.physics.destroy_entity(world, rock.entity);
        }
    }

    /// Add points to ship `ship_index`, unlock score tiers, and grant that
    /// ship a bonus life at every `EXTRA_LIFE_EVERY` crossing (each threshold
    /// pays out exactly once, per ship).
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
        while ship.score >= ship.next_extra_life_at {
            ship.lives += 1;
            ship.next_extra_life_at += EXTRA_LIFE_EVERY;
        }
    }

    /// Push the current `chaos_mode`'s look onto the live scenery:
    /// background tint and a fresh grid. Wireframe colors read the theme
    /// every frame, so they follow automatically.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(bg) = self.background {
            if let Some(s) = world.get_mut::<Sprite>(bg) {
                s.color = theme.bg_color;
            }
        }
        self.grid = Some(default_playfield_grid(&theme));
    }
}
