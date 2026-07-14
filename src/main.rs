mod achievements;
mod constants;
mod drawing;
mod effects;
mod gameplay;
#[cfg(test)]
mod gameplay_tests;
mod menu;
mod spawning;
mod types;
mod wireframe;

use engine_core::prelude::*;
use constants::*;
use spawning::*;
use types::*;

impl Game for AsteroidsGame {
    fn init(&mut self, ctx: &mut GameContext) {
        let font_path = engine_core::game_root!().join("assets/fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        achievements::register_all(ctx.achievements);

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.tex_id = tex.id;

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        self.background = Some(spawn_background(
            ctx.world, tex.id, theme.bg_color, Vec2::new(WIN_W, WIN_H)));

        // The ship lives for the whole session (it's a wireframe, so it's
        // simply not drawn on the menu screens); rocks and bullets spawn
        // fresh on every `start_game()`.
        self.ship = Some(spawn_ship(ctx.world));

        self.grid = Some(default_playfield_grid(&theme));
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frame_count = self.frame_count.wrapping_add(1);

        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::ModeSelect { selection } => self.update_mode_select_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }

        self.update_entity_visibility(ctx);
        self.draw_ui(ctx);
    }
}

fn main() {
    // Anchor assets and saves to the game's directory so launching from any
    // working directory behaves the same.
    let root = engine_core::game_root!();
    let config = GameConfig::new("Insiculous Asteroids")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        .with_asset_base_path(root.join("assets").to_string_lossy())
        .with_achievement_save_path(root.join("saves/asteroids_achievements.json").to_string_lossy());

    // With `--features editor` the game runs inside the scene editor
    // (hierarchy, inspector, gizmos, play/pause/stop, collider overlay);
    // without it the game runs bare. Same game code either way.
    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(AsteroidsGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    run_game(AsteroidsGame::default(), config).unwrap();
}
