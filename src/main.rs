use insiculous_asteroids::{game_config, AsteroidsGame};

fn main() {
    // Anchor assets and saves to the game's directory so launching from any
    // working directory behaves the same.
    let root = engine_core::game_root!();
    let config = game_config(&root.join("assets").to_string_lossy())
        .with_achievement_save_path(root.join("saves/asteroids_achievements.json").to_string_lossy())
        .with_input_settings_path(root.join("saves/input_settings.json").to_string_lossy())
        .with_score_save_path(root.join("saves/asteroids_scores.json").to_string_lossy());

    // With `--features editor` the game runs inside the scene editor
    // (hierarchy, inspector, gizmos, play/pause/stop, collider overlay);
    // without it the game runs bare. Same game code either way.
    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(AsteroidsGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    {
        // One design size: wrapping, the border spawns, the grid and the star tiles all
        // read the 800x600 field, and the world never scales with a native window, so a
        // bigger one would wrap meatieroids mid-screen and run past the stars. Only the
        // bare game is locked — the editor needs room for its panels, and the web page
        // scales the whole canvas instead.
        let mut config = config;
        config.resizable = false;
        engine_core::prelude::run_game(AsteroidsGame::default(), config).unwrap();
    }
}
