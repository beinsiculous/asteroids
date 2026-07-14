//! Match lifecycle: state-machine input during play, starting/ending a
//! match, scoring, and pushing the chaos theme onto live entities.

use engine_core::prelude::*;

use crate::constants::*;
use crate::types::*;

impl AsteroidsGame {
    /// Keys that change the game state while the simulation screens are up.
    pub(crate) fn handle_state_input(&mut self, ctx: &mut GameContext) {
        match &self.state {
            GameState::Playing => {
                if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            GameState::GameOver => {
                if ctx.input.is_key_just_pressed(KeyCode::Space)
                    || ctx.input.is_key_just_pressed(KeyCode::Enter)
                {
                    self.start_game(ctx);
                } else if ctx.input.is_key_just_pressed(KeyCode::Escape) {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
        }
    }

    /// Reset every counter, clear the field, park the ship at the center,
    /// and spawn wave 1. Called from mode select and game-over restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.score = 0;
        self.lives = STARTING_LIVES;
        self.wave = 1;
        self.next_extra_life_at = EXTRA_LIFE_EVERY;
        self.fire_cooldown = 0.0;
        self.invincibility = 0.0;
        self.wave_clear_timer = 0.0;
        self.thrusting = false;
        self.shot_streak = 0;
        self.waves_without_death = 0;
        self.last_kill_time = f32::NEG_INFINITY;
        self.play_time = 0.0;

        self.destroy_all_bullets(ctx.world);
        self.destroy_all_asteroids(ctx.world);
        self.reset_ship_pose(ctx.world);
        self.apply_theme(ctx.world);
        self.spawn_wave(ctx.world, Vec2::ZERO);
        self.state = GameState::Playing;
    }

    /// Out of ships. The rocks keep drifting behind the game-over overlay;
    /// the next start clears them.
    pub(crate) fn finish_game(&mut self, ctx: &mut GameContext) {
        self.destroy_all_bullets(ctx.world);
        self.thrusting = false;
        self.state = GameState::GameOver;
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.destroy_all_bullets(world);
        self.destroy_all_asteroids(world);
        self.thrusting = false;
        self.state = GameState::TitleScreen { selection: 0 };
    }

    pub(crate) fn destroy_all_bullets(&mut self, world: &mut World) {
        for bullet in self.bullets.drain(..).collect::<Vec<_>>() {
            self.physics.destroy_entity(world, bullet);
        }
    }

    pub(crate) fn destroy_all_asteroids(&mut self, world: &mut World) {
        for rock in self.asteroids.drain(..).collect::<Vec<_>>() {
            self.physics.destroy_entity(world, rock.entity);
        }
    }

    /// Add points, unlock score tiers, and grant a bonus ship at every
    /// `EXTRA_LIFE_EVERY` crossing (each threshold pays out exactly once).
    pub(crate) fn award_points(&mut self, achievements: &mut AchievementManager, points: u32) {
        self.score += points;
        if self.score >= SCORE_TIER_1 {
            achievements.unlock(crate::achievements::SCORE_10K);
        }
        if self.score >= SCORE_TIER_2 {
            achievements.unlock(crate::achievements::SCORE_30K);
        }
        while self.score >= self.next_extra_life_at {
            self.lives += 1;
            self.next_extra_life_at += EXTRA_LIFE_EVERY;
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
