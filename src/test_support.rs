//! Shared test fixtures.
//!
//! The synced sheets are read back through the engine's own GPU-free load path, so a
//! test can check the game's clip names against the committed art without a window, a
//! GPU or a `GameContext`. The fixtures also drive the whole game through the engine's
//! harness: the real frame, with no window.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine_core::assets::sprite_sheet::{prepare_sheet, PreparedSheet};
use engine_core::prelude::*;
use engine_core::test_support::{DrawCommand, GameHarness, InputEvent};

use crate::constants::BULLET_SPEED;
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

/// Step one frame with `events` arriving before it.
pub(crate) fn step_with(harness: &mut GameHarness<AsteroidsGame>, events: &[InputEvent]) {
    harness.step(FRAME, events);
}

/// Frames with wall time between them, so the song's virtual clock renders blocks and the
/// conductor drains what the frames sent.
pub(crate) fn settle(harness: &mut GameHarness<AsteroidsGame>) {
    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
    }
}

/// Step with wall time between frames until `done` holds, failing past `limit`.
pub(crate) fn poll(
    harness: &mut GameHarness<AsteroidsGame>,
    limit: Duration,
    what: &str,
    done: impl Fn(&GameHarness<AsteroidsGame>) -> bool,
) {
    let started = Instant::now();
    while !done(harness) {
        assert!(started.elapsed() < limit, "{what} within {limit:?}");
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
    }
}

/// Where the pause menu drew `label` in the last frame.
pub(crate) fn drawn_at(harness: &GameHarness<AsteroidsGame>, label: &str) -> Vec2 {
    harness.ui_commands().iter().find_map(|command| match command {
        DrawCommand::Text { data, .. } if data.text.contains(label) => {
            Some(data.position + Vec2::new(4.0, data.height / 2.0))
        }
        _ => None,
    }).unwrap_or_else(|| panic!("'{label}' was drawn"))
}

pub(crate) fn key(harness: &mut GameHarness<AsteroidsGame>, code: KeyCode) {
    harness.step(FRAME, &[press(code)]);
    harness.step(FRAME, &[release(code)]);
}

pub(crate) fn source_of(harness: &GameHarness<AsteroidsGame>, entity: EntityId) -> Option<SourceId> {
    harness.sound_sources().source_of(entity)
}

pub(crate) fn ship(harness: &GameHarness<AsteroidsGame>, index: usize) -> EntityId {
    harness.game().ships[index].entity.expect("the ship is alive")
}

/// The match stops running while the song plays on: no ship, no break, no wave, so a test
/// that waits on wall time hears only what it sent.
pub(crate) fn hold_the_match(harness: &mut GameHarness<AsteroidsGame>) {
    harness.context(|game, _| game.state = GameState::GameOver);
}

/// A match whose field holds one still meatieroid far in a corner, so no wave clears and
/// spawns under a test, with what the match start sent drained.
pub(crate) fn anchored(mode: GameMode, chaos: ChaosMode) -> (GameHarness<AsteroidsGame>, EntityId) {
    let mut harness = empty_field(mode, chaos);
    let anchor = place_meatieroid(&mut harness, AsteroidSize::Small, Vec2::new(-340.0, 250.0));
    settle(&mut harness);
    (harness, anchor)
}

/// Destroy every meatieroid but `keep`: a break's children leave the field.
pub(crate) fn clear_but(harness: &mut GameHarness<AsteroidsGame>, keep: EntityId) {
    harness.context(|game, ctx| {
        let (kept, gone): (Vec<Asteroid>, Vec<Asteroid>) = game.asteroids.drain(..).partition(|rock| rock.entity == keep);
        game.asteroids = kept;
        for rock in gone {
            game.physics.destroy_entity(ctx.world, rock.entity);
        }
    });
}

/// A shot of ship `index` flying up into `rock` from 30 px under its body's edge, so two
/// shots fired together reach meatieroids of any sizes on the same frame.
pub(crate) fn shoot_at(harness: &mut GameHarness<AsteroidsGame>, index: usize, rock: EntityId) {
    let target = position_of(harness, rock);
    harness.context(|game, ctx| {
        let size = game.asteroids.iter().find(|candidate| candidate.entity == rock).expect("a live meatieroid").size;
        let below = Vec2::new(0.0, size.radius() + 30.0);
        game.spawn_bullet(ctx.world, index, target - below, Vec2::new(0.0, BULLET_SPEED));
    });
}

/// Step until a frame's trace holds a trigger, and return it.
pub(crate) fn next_trace(harness: &mut GameHarness<AsteroidsGame>) -> Vec<&'static str> {
    for _ in 0..60 {
        harness.step(FRAME, &[]);
        let trace = harness.game().music.triggered.clone();
        if !trace.is_empty() {
            return trace;
        }
    }
    panic!("no trigger in a second of frames");
}
