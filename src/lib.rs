//! Meatieroids — game crate: asteroids, re-skinned as Deion's Flying Funk among the
//! roided-out meatballs.
//!
//! The library owns the whole game (`AsteroidsGame` + its `Game` impl) so both
//! entry points stay thin: `main.rs` (native window, filesystem saves,
//! optional editor) and `web_entry.rs` (wasm-bindgen start: fetch assets,
//! then the same `run_game`). This split also keeps `editor_integration`
//! behind the `editor` feature in both entry points — `main.rs` for the native
//! window, `web_entry.rs` for the browser's editor bundle.

mod achievements;
mod art;
#[cfg(test)]
mod art_tests;
mod constants;
mod drawing;
mod effects;
mod gameplay;
#[cfg(test)]
mod gameplay_tests;
#[cfg(test)]
mod harness_tests;
mod menu;
mod spawning;
#[cfg(test)]
mod test_support;
mod types;

#[cfg(target_arch = "wasm32")]
mod web_entry;

use engine_core::prelude::*;
use art::*;
use constants::*;
use spawning::{spawn_backdrop, spawn_space};
use types::*;

pub use types::AsteroidsGame;

/// Load a synced sheet; a missing or malformed one is a broken build, not a state
/// the game can play on.
fn load_sheet(assets: &mut AssetManager, spec: &SheetSpec) -> SpriteSheet {
    assets
        .load_sprite_sheet(spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

/// The shared `GameConfig` for every target. Entry points add their own
/// platform extras on top (native: save paths anchored to the game dir;
/// web: nothing — no save paths means in-memory achievements and default
/// input bindings).
///
/// `asset_base` must be an ANCHORED base: native callers pass an absolute
/// path (`main.rs` derives it from `game_root!()` so the cwd never
/// matters); the web entry passes the deploy URL base. Passing a bare
/// relative path like `"assets"` would silently resolve against the
/// current working directory.
pub fn game_config(asset_base: &str) -> GameConfig {
    let [red, green, blue, alpha] = night_clear_color();
    GameConfig::new("Meatieroids")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(red, green, blue, alpha)
        .with_fps(60)
        .with_pixel_snap(true)
        .with_startup_splashes(STARTUP_CARDS)
        .with_window_icon(WINDOW_ICON)
        .with_asset_base_path(asset_base)
}

impl Game for AsteroidsGame {
    fn register_achievements(&self, achievements: &mut AchievementManager, _strings: &Strings) {
        achievements::register_all(achievements);
    }

    fn init(&mut self, ctx: &mut GameContext) {
        // Resolve against the configured asset base so the same relative
        // path works natively (game dir) and on the web (VFS keys).
        let font_path = std::path::Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        // Every particle burst draws with it; without it they would all sample texture 0.
        self.tex_id = ctx.assets
            .create_solid_color(1, 1, [255, 255, 255, 255])
            .unwrap_or_else(|error| panic!("the particles' white texture does not create: {error}"))
            .id;

        // Every sheet's path and cell is in `art.rs`; each PNG and its `.sheet.ron`
        // sidecar is a synced copy under `assets/sprites/`.
        let sheets = &mut self.sheets;
        for (field, spec) in [
            (&mut sheets.flying_funk, &FLYING_FUNK),
            (&mut sheets.flying_funk_player_two, &FLYING_FUNK_PLAYER_TWO),
            (&mut sheets.meatieroid_large, &MEATIEROID_LARGE),
            (&mut sheets.meatieroid_medium, &MEATIEROID_MEDIUM),
            (&mut sheets.meatieroid_small, &MEATIEROID_SMALL),
            (&mut sheets.icicle, &ICICLE),
            (&mut sheets.ice_chip, &ICE_CHIP),
            (&mut sheets.space, &SPACE),
        ] {
            *field = load_sheet(ctx.assets, spec);
        }

        // The stars and the grid stay up under the menus. Ships, meatieroids and shots
        // spawn fresh in `start_game()` — the ship count depends on the mode the player
        // picks, and dead ships are despawned mid-match.
        self.space = Some(spawn_space(ctx.world, &self.sheets.space));
        self.backdrop = Some(spawn_backdrop(ctx.world, &ChaosTheme::for_mode(self.chaos_mode)));
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
