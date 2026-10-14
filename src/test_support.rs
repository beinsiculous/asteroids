//! Shared test fixtures.
//!
//! The synced sheets are read back through the engine's own GPU-free load path, so a
//! test can check the game's clip names against the committed art without a window, a
//! GPU or a `GameContext`. The fixtures also drive the whole game through the engine's
//! harness: the real frame, with no window.

use std::path::{Path, PathBuf};

use engine_core::assets::sprite_sheet::{prepare_sheet, PreparedSheet};
use engine_core::prelude::*;
use engine_core::test_support::{GameHarness, InputEvent};

use crate::types::*;

pub(crate) const FRAME: f32 = 1.0 / 60.0;

/// The synced art's directory, anchored to the crate so the working directory never
/// matters.
pub(crate) fn asset_base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// A synced sheet read through the engine's own GPU-free load path.
pub(crate) fn sidecar(spec: &SheetSpec) -> PreparedSheet {
    prepare_sheet(&asset_base(), spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

/// The title screen's world: the game's own config over the synced art, with no save
/// paths so achievements and scores stay in memory, and one frame run so `init` has
/// loaded the sheets and spawned the stars and the grid.
pub(crate) fn title_harness() -> GameHarness<AsteroidsGame> {
    let base = asset_base();
    let config = crate::game_config(base.to_str().expect("the asset path is UTF-8"));
    let mut harness = GameHarness::new(AsteroidsGame::default(), config);
    harness.step(FRAME, &[]);
    harness
}

/// A match started as the mode select does it, in `mode` and `chaos`, with the wave's
/// meatieroids cleared away so a test places only what it needs.
pub(crate) fn empty_field(mode: GameMode, chaos: ChaosMode) -> GameHarness<AsteroidsGame> {
    let mut harness = title_harness();
    harness.context(|game, ctx| {
        game.mode = mode;
        game.chaos_mode = chaos;
        ctx.chaos_mode = chaos;
        game.start_game(ctx);
        game.destroy_all_asteroids(ctx.world);
    });
    harness
}

/// Place a still meatieroid of `size` at `position`, born as a wave's are.
pub(crate) fn place_meatieroid(harness: &mut GameHarness<AsteroidsGame>, size: AsteroidSize, position: Vec2) -> EntityId {
    harness.context(|game, ctx| {
        game.spawn_asteroid(ctx.world, size, position, Vec2::ZERO, 0.0, crate::spawning::wave_machine())
    })
}

/// The world position of `entity`, which must exist.
pub(crate) fn position_of(harness: &GameHarness<AsteroidsGame>, entity: EntityId) -> Vec2 {
    harness.world().get::<Transform2D>(entity).map(|transform| transform.position).expect("the entity exists")
}

/// The state `entity`'s clip machine is in.
pub(crate) fn machine_state(harness: &GameHarness<AsteroidsGame>, entity: EntityId) -> String {
    harness.world().get::<ClipStateMachine>(entity).map(|machine| machine.state().to_string()).expect("a clip machine")
}

/// Whether `entity` is still in the world.
pub(crate) fn alive(harness: &GameHarness<AsteroidsGame>, entity: EntityId) -> bool {
    harness.world().entities().contains(&entity)
}

pub(crate) fn press(key: KeyCode) -> InputEvent {
    InputEvent::KeyPressed(key)
}

pub(crate) fn release(key: KeyCode) -> InputEvent {
    InputEvent::KeyReleased(key)
}
