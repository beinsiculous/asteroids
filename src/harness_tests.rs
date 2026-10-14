//! The whole game driven through the engine's harness: the real frame, the real spawn
//! paths and the synced art, with no window. Each test places what it needs on an
//! emptied field.

use engine_core::prelude::*;
use std::f32::consts::{FRAC_PI_2, TAU};

use crate::art::*;
use crate::test_support::*;
use crate::types::*;

const HEADING_STEP: f32 = TAU / HEADINGS as f32;

/// The ship's drawn clip after the last frame.
fn ship_clip(harness: &engine_core::test_support::GameHarness<AsteroidsGame>, index: usize) -> String {
    let entity = harness.game().ships[index].entity.expect("the ship is alive");
    harness.world().get::<SpriteAnimation>(entity)
        .and_then(|animation| animation.current_clip.clone())
        .expect("the ship has a pose")
}

/// A meatieroid tumbles about its body, not its cell: the engine draws the cell's centre
/// at the entity plus the sprite offset turned by the entity's rotation, and from there
/// the body's centre — the offset's mirror in the cell — lands back on the entity, at
/// every angle. Unsnapped: this is the placement, before pixel snapping rounds it.
#[test]
fn a_tumbling_meatieroid_turns_about_its_body() {
    let mut harness = empty_field(GameMode::SinglePlayer, ChaosMode::Normal);
    let rock = harness.context(|game, ctx| {
        game.spawn_asteroid(ctx.world, AsteroidSize::Large, Vec2::new(200.0, 120.0), Vec2::ZERO, 1.4, crate::spawning::wave_machine())
    });
    let mut turned: f32 = 0.0;
    for _ in 0..140 {
        harness.step(FRAME, &[]);
        let transform = *harness.world().get::<Transform2D>(rock).expect("the meatieroid lives");
        let offset = harness.world().get::<Sprite>(rock).expect("drawn").offset;
        assert_eq!(offset, MEATIEROID_LARGE.sprite_offset(), "the offset is the sheet's, unchanged by the tumble");
        let turn = Vec2::from_angle(transform.rotation);
        let cell_centre = transform.position + turn.rotate(offset);
        let body_centre = cell_centre + turn.rotate(-offset);
        assert!(body_centre.distance(transform.position) < 1e-3, "at {} rad the body drew off the entity", transform.rotation);
        turned = turned.max(transform.rotation.abs());
    }
    assert!(turned > std::f32::consts::PI * 0.9, "the meatieroid turned only {turned} rad");
}

/// A shot leaves along the drawn heading: fired on each diagonal, it hits a small
/// meatieroid placed on that line.
#[test]
fn a_shot_on_each_diagonal_hits_what_lies_on_it() {
    for heading_index in [2usize, 6, 10, 14] {
        let mut harness = empty_field(GameMode::SinglePlayer, ChaosMode::Normal);
        let angle = heading_index as f32 * HEADING_STEP;
        let direction = Vec2::new(angle.cos(), angle.sin());
        harness.context(|game, _| game.ships[0].heading = angle + 0.05);
        let target = place_meatieroid(&mut harness, AsteroidSize::Small, direction * 150.0);

        harness.step(FRAME, &[press(KeyCode::Space)]);
        harness.step(FRAME, &[release(KeyCode::Space)]);
        for _ in 0..40 {
            harness.step(FRAME, &[]);
        }
        assert!(!alive(&harness, target), "heading {heading_index:02}: the shot missed");
        assert_eq!(harness.game().ships[0].score, AsteroidSize::Small.score(), "heading {heading_index:02}");
        assert_eq!(ship_clip(&harness, 0), IDLE_HEADINGS[heading_index], "the ship is drawn on the heading it fired along");
    }
}

/// A split's children are born flexing, settle into `idle`, and are still there a second
/// later — the flex never despawns them. The shattered parent's `die` plays out and ends.
#[test]
fn split_children_flex_then_idle_and_live_on() {
    let mut harness = empty_field(GameMode::SinglePlayer, ChaosMode::Normal);
    let parent = place_meatieroid(&mut harness, AsteroidSize::Large, Vec2::new(-250.0, 150.0));
    harness.context(|game, ctx| game.shatter_asteroid(ctx, 0, parent));
    let children: Vec<EntityId> = harness.game().asteroids.iter().map(|rock| rock.entity).collect();
    assert_eq!(children.len(), 2);
    let die = *harness.game().transient_visuals.last().expect("the parent's die plays");
    assert_eq!(machine_state(&harness, die), DIE);
    assert!(harness.world().get::<Collider>(die).is_none(), "a die never collides");

    harness.step(FRAME, &[]);
    for &child in &children {
        assert_eq!(machine_state(&harness, child), ATTACK, "a child is born flexing");
    }
    for _ in 0..30 {
        harness.step(FRAME, &[]);
    }
    for &child in &children {
        assert_eq!(machine_state(&harness, child), IDLE, "the flex settles into idle");
    }
    for _ in 0..60 {
        harness.step(FRAME, &[]);
    }
    for &child in &children {
        assert!(alive(&harness, child), "a child outlives its flex");
    }
    assert!(!alive(&harness, die), "the die ended itself");
    assert!(harness.game().transient_visuals.is_empty(), "an ended one-shot is forgotten without a restart");
}

/// A lost life's crash plays detached where the ship was, turned to its drawn heading;
/// the ship itself respawns at its spawn point on heading 04 with its body unturned.
#[test]
fn a_lost_life_crashes_where_the_ship_was_and_respawns_facing_up() {
    let mut harness = empty_field(GameMode::SinglePlayer, ChaosMode::Normal);
    let crash_spot = Vec2::new(120.0, -60.0);
    harness.context(|game, ctx| {
        game.ships[0].heading = 3.0 * HEADING_STEP;
        let ship = game.ships[0].entity.expect("the ship is alive");
        ctx.world.get_mut::<Transform2D>(ship).expect("a transform").position = crash_spot;
    });
    harness.step(FRAME, &[]);
    let lives = harness.game().ships[0].lives;
    let ship_spot = position_of(&harness, harness.game().ships[0].entity.expect("alive"));
    place_meatieroid(&mut harness, AsteroidSize::Small, ship_spot);
    harness.step(FRAME, &[]);

    assert_eq!(harness.game().ships[0].lives, lives - 1, "the meatieroid took a life");
    let crash = *harness.game().transient_visuals.last().expect("the crash plays");
    assert_eq!(machine_state(&harness, crash), CRASH);
    let crash_transform = *harness.world().get::<Transform2D>(crash).expect("the crash is placed");
    assert!(crash_transform.position.distance(ship_spot) < 1e-3, "the crash plays where the ship was");
    assert!((crash_transform.rotation - 3.0 * HEADING_STEP).abs() < 1e-5, "the crash turns to the drawn heading");
    assert!(harness.world().get::<Collider>(crash).is_none(), "a crash never collides");
    assert!(harness.world().get::<RigidBody>(crash).is_none(), "a crash is detached from physics");

    let ship = harness.game().ships[0].entity.expect("a life remains");
    assert_eq!(harness.game().ships[0].heading, FRAC_PI_2, "the ship respawns facing up");
    harness.step(FRAME, &[]);
    assert_eq!(ship_clip(&harness, 0), "idle_04");
    assert_eq!(harness.world().get::<Transform2D>(ship).map(|t| t.rotation), Some(0.0), "the body never turns");
    assert!(position_of(&harness, ship).length() < 1.0, "the ship respawns at the centre");
}

/// Player 2 flies the second Funk and fires ice chips; player 1 the first and icicles.
#[test]
fn coop_ships_fly_their_pilots_funks() {
    let mut harness = empty_field(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    harness.step(FRAME, &[]);
    for (index, pilot) in [(0, Pilot::Deion), (1, Pilot::Cubert)] {
        let ship = harness.game().ships[index].entity.expect("alive");
        let name = harness.world().get::<Name>(ship).map(|name| name.as_str().to_string());
        assert_eq!(name.as_deref(), Some(pilot.art().name));
        assert_eq!(ship_clip(&harness, index), "idle_04");
    }
}

/// A restart or a quit ends every one-shot still playing.
#[test]
fn a_restart_or_a_quit_ends_every_one_shot() {
    for quit in [false, true] {
        let mut harness = empty_field(GameMode::SinglePlayer, ChaosMode::Normal);
        let parent = place_meatieroid(&mut harness, AsteroidSize::Medium, Vec2::new(-250.0, 150.0));
        harness.context(|game, ctx| game.shatter_asteroid(ctx, 0, parent));
        let ship_spot = position_of(&harness, harness.game().ships[0].entity.expect("alive"));
        place_meatieroid(&mut harness, AsteroidSize::Small, ship_spot);
        harness.step(FRAME, &[]);
        let one_shots = harness.game().transient_visuals.clone();
        assert_eq!(one_shots.len(), 2, "a die and a crash are playing");

        harness.context(|game, ctx| if quit { game.reset_to_title(ctx.world) } else { game.start_game(ctx) });
        assert!(harness.game().transient_visuals.is_empty());
        for entity in one_shots {
            assert!(!alive(&harness, entity), "a one-shot outlived the {}", if quit { "quit" } else { "restart" });
        }
    }
}

/// A meatieroid drifting slower than physics' sleep threshold (40 px/s) keeps drifting
/// and turning: in space nothing comes to rest, and one that fell asleep would stop dead
/// for the rest of the wave.
#[test]
fn a_slow_meatieroid_never_stops() {
    let mut harness = empty_field(GameMode::SinglePlayer, ChaosMode::Normal);
    let velocity = Vec2::new(35.0, 0.0);
    let rock = harness.context(|game, ctx| {
        game.spawn_asteroid(ctx.world, AsteroidSize::Large, Vec2::new(-200.0, -150.0), velocity, 0.3, crate::spawning::wave_machine())
    });
    for _ in 0..360 {
        harness.step(FRAME, &[]);
    }
    let (drift, spin) = harness.game().physics.get_body_velocity(rock).expect("the meatieroid has a body");
    assert!((drift - velocity).length() < 0.01 && (spin - 0.3).abs() < 1e-3, "after six seconds it moves at {drift:?}, turning {spin}");
}
