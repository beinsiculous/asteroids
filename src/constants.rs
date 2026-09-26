//! Every tunable in one place. Sizes are world pixels; sprite scales divide
//! by `RENDER_UNIT` at the spawn site. Colliders take absolute pixels.

use glam::Vec4;

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;

// --- Ship ---
/// Distance from ship center to the nose, world pixels.
pub(crate) const SHIP_SIZE: f32 = 16.0;
/// Forgiving hit circle, smaller than the drawn hull.
pub(crate) const SHIP_COLLIDER_RADIUS: f32 = 10.0;
pub(crate) const SHIP_TURN_SPEED: f32 = 4.2; // rad/s
pub(crate) const SHIP_THRUST: f32 = 340.0; // px/s^2
pub(crate) const SHIP_MAX_SPEED: f32 = 380.0; // px/s
/// Fraction of velocity shed per second while coasting (classic drift decay).
pub(crate) const SHIP_DRAG: f32 = 0.45;
pub(crate) const SHIP_EMISSIVE: f32 = 1.8;
/// Player 2's hull/flame color in co-op (player 1 uses the chaos accent).
pub(crate) const SHIP2_COLOR: Vec4 = Vec4::new(0.35, 1.0, 0.55, 1.0);
/// Half the gap between the two co-op ships at spawn (∓ this on the x axis).
pub(crate) const COOP_SHIP_SPAWN_X: f32 = 80.0;
pub(crate) const FLAME_COLOR: Vec4 = Vec4::new(1.0, 0.6, 0.15, 1.0);
pub(crate) const FLAME_EMISSIVE: f32 = 2.2;
pub(crate) const STARTING_LIVES: u32 = 3;
/// Seconds of post-respawn invincibility (halved in Insane-family modes).
pub(crate) const RESPAWN_INVINCIBILITY: f32 = 3.0;
pub(crate) const INSANE_INVINCIBILITY_MULT: f32 = 0.5;
pub(crate) const INVINCIBILITY_BLINK_HZ: f32 = 8.0;
pub(crate) const EXTRA_LIFE_EVERY: u32 = 10_000;

// --- Bullets ---
pub(crate) const BULLET_SPEED: f32 = 520.0;
pub(crate) const BULLET_RADIUS: f32 = 3.0;
/// Sprite square edge, world pixels.
pub(crate) const BULLET_SIZE_PX: f32 = 6.0;
/// ~570 px of travel — under one full screen crossing even with wrap.
pub(crate) const BULLET_LIFETIME: f32 = 1.1;
pub(crate) const FIRE_COOLDOWN: f32 = 0.22;
pub(crate) const MAX_LIVE_BULLETS: usize = 4;
pub(crate) const MUZZLE_OFFSET: f32 = SHIP_SIZE + 4.0;
pub(crate) const BULLET_EMISSIVE: f32 = 2.5;
pub(crate) const BULLET_COLOR: Vec4 = Vec4::new(1.0, 0.95, 0.6, 1.0);

// --- Asteroids (index order: Large, Medium, Small) ---
pub(crate) const ASTEROID_RADII: [f32; 3] = [40.0, 22.0, 12.0];
/// Small rocks are the hardest targets, so they pay the most.
pub(crate) const ASTEROID_SCORES: [u32; 3] = [20, 50, 100];
pub(crate) const ASTEROID_SPEED_RANGES: [(f32, f32); 3] =
    [(40.0, 90.0), (70.0, 140.0), (110.0, 200.0)];
pub(crate) const INSANE_SPEED_MULT: f32 = 1.6;
/// Spin drawn from [-max, max] rad/s per rock.
pub(crate) const ASTEROID_SPIN_MAX: f32 = 1.4;
pub(crate) const ASTEROID_VERTS: usize = 10;
/// Collider radius as a fraction of the nominal (mean polygon) radius.
pub(crate) const ASTEROID_COLLIDER_SCALE: f32 = 0.9;
pub(crate) const ASTEROID_EMISSIVE: f32 = 1.1;

// --- Splitting ---
pub(crate) const SPLIT_COUNT: usize = 2;
pub(crate) const RIDICULOUS_LARGE_SPLIT_COUNT: usize = 3;
pub(crate) const SPLIT_SPEED_MULT: f32 = 1.25;
/// Radians between adjacent child directions (before jitter).
pub(crate) const SPLIT_SPREAD: f32 = 0.9;

// --- Waves ---
pub(crate) const WAVE_BASE_ASTEROIDS: usize = 4;
pub(crate) const WAVE_MAX_ASTEROIDS: usize = 9;
pub(crate) const RIDICULOUS_EXTRA_ASTEROIDS: usize = 1;
/// New rocks never spawn closer than this to the ship.
pub(crate) const SAFE_SPAWN_DIST: f32 = 160.0;
pub(crate) const WAVE_CLEAR_DELAY: f32 = 2.0;

// --- Achievements thresholds ---
pub(crate) const SHARPSHOOTER_TARGET: u32 = 10;
pub(crate) const CLOSE_CALL_DIST: f32 = 60.0;
pub(crate) const UNTOUCHABLE_WAVES: u32 = 3;
pub(crate) const DOUBLE_TAP_WINDOW: f32 = 0.5;
pub(crate) const WAVE_MILESTONE: u32 = 5;
pub(crate) const SCORE_TIER_1: u32 = 10_000;
pub(crate) const SCORE_TIER_2: u32 = 30_000;

// --- Grid impulses (strength, radius) ---
pub(crate) const GRID_IMPULSE_BREAK: [(f32, f32); 3] =
    [(320.0, 130.0), (220.0, 95.0), (150.0, 60.0)];
pub(crate) const GRID_IMPULSE_SHIP_DEATH: (f32, f32) = (700.0, 160.0);

// --- the startup cards and the window icon ---------------------------------------
// Synced from deion_assets like every sheet (`assets/sprites/sync.list`).

/// The cards every Insiculous game opens on, in order: the studio's, then the
/// engine's. The engine shows them before `init` (`GameConfig::with_startup_splashes`).
pub(crate) const STARTUP_CARDS: [&str; 2] = [
    "sprites/ai_be_insiculous_320x192.png",
    "sprites/ai_insiculous_2d_maxwell_splash_320x192.png",
];
/// The engine's icon: the window's until the game draws one of its own.
pub(crate) const WINDOW_ICON: &str = "sprites/ai_insiculous_2d_maxwell_icon_64x64.png";

#[cfg(test)]
mod startup_card_tests {
    use super::*;
    use engine_core::{AssetConfig, AssetManager};

    #[test]
    fn the_startup_cards_are_the_studio_then_the_engine_on_their_contract_backdrops() {
        let assets_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let config = crate::game_config(assets_directory.to_str().expect("the asset path is UTF-8"));
        assert_eq!(config.startup_splashes, STARTUP_CARDS, "the studio's card, then the engine's");
        assert_eq!(config.window_icon.as_deref(), Some(WINDOW_ICON));

        // The corners are BRANDING.md's named fills, which the engine letterboxes each
        // card in; a redrawn backdrop that drifts from the contract fails here.
        let assets = AssetManager::headless(AssetConfig::from(&config));
        for (path, size, corner) in [
            (STARTUP_CARDS[0], (320, 192), Some([0x14, 0x10, 0x1F, 0xFF])),
            (STARTUP_CARDS[1], (320, 192), Some([0x4A, 0x44, 0x58, 0xFF])),
            (WINDOW_ICON, (64, 64), None),
        ] {
            let image = assets.image_backdrop(path).unwrap_or_else(|| panic!("{path} is synced"));
            assert_eq!((image.size.x, image.size.y), size, "{path}");
            if let Some(corner) = corner {
                assert_eq!(image.corner.to_rgba8(), corner, "{path}");
            }
        }
    }
}
