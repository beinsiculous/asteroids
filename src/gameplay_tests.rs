//! Behavioral tests: pure rules (flight model, wrapping, splitting, waves)
//! and headless physics simulations proving every rapier pair the game
//! depends on, using the exact spawn recipes.

use engine_core::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::constants::*;
use crate::gameplay::*;
use crate::menu::mode_hint;
use crate::spawning::spawn_ship;
use crate::types::*;
use crate::wireframe::*;

const DT: f32 = 1.0 / 60.0;
const HALF: Vec2 = Vec2::new(WIN_W / 2.0, WIN_H / 2.0);

// --- Screen wrap ---

#[test]
fn wrap_position_passes_through_inbounds_points() {
    let p = Vec2::new(120.0, -200.0);
    assert_eq!(wrap_position(p, HALF, 10.0), p);
}

#[test]
fn wrap_position_wraps_horizontally_preserving_y() {
    let margin = 12.0;
    let off_right = Vec2::new(HALF.x + margin + 1.0, 42.0);
    let wrapped = wrap_position(off_right, HALF, margin);
    assert_eq!(wrapped.y, 42.0);
    assert_eq!(wrapped.x, -(HALF.x + margin));

    let off_left = Vec2::new(-(HALF.x + margin + 1.0), -13.0);
    let wrapped = wrap_position(off_left, HALF, margin);
    assert_eq!(wrapped.x, HALF.x + margin);
    assert_eq!(wrapped.y, -13.0);
}

#[test]
fn wrap_position_wraps_vertically_only_beyond_margin() {
    let margin = 20.0;
    // Just past the window edge but inside the margin: still visible-ish,
    // no wrap yet (the entity exits fully before reappearing).
    let barely_out = Vec2::new(0.0, HALF.y + margin - 1.0);
    assert_eq!(wrap_position(barely_out, HALF, margin), barely_out);

    let fully_out = Vec2::new(0.0, HALF.y + margin + 1.0);
    assert_eq!(wrap_position(fully_out, HALF, margin), Vec2::new(0.0, -(HALF.y + margin)));
}

// --- Flight model ---

#[test]
fn thrust_accelerates_along_heading() {
    let (vel, _) = integrate_ship(Vec2::ZERO, 0.0, 0.0, true, DT);
    assert!(vel.x > 0.0, "heading 0 = +x, thrust must push +x");
    assert!(vel.y.abs() < 1e-4);

    let (vel, _) = integrate_ship(Vec2::ZERO, FRAC_PI_2, 0.0, true, DT);
    assert!(vel.y > 0.0, "heading pi/2 = +y, thrust must push +y");
    assert!(vel.x.abs() < 1e-3);
}

#[test]
fn speed_clamps_at_max() {
    let mut vel = Vec2::ZERO;
    for _ in 0..3000 {
        (vel, _) = integrate_ship(vel, 0.3, 0.0, true, DT);
    }
    assert!(vel.length() <= SHIP_MAX_SPEED + 0.001, "speed {} over cap", vel.length());
    assert!(vel.length() > SHIP_MAX_SPEED * 0.95, "thrust should reach the cap");
}

#[test]
fn drag_decays_coasting_velocity_without_reversing() {
    let mut vel = Vec2::new(200.0, 0.0);
    let start = vel.length();
    for _ in 0..120 {
        (vel, _) = integrate_ship(vel, 0.0, 0.0, false, DT);
        assert!(vel.x >= 0.0, "drag must never reverse the drift direction");
    }
    assert!(vel.length() < start, "coasting must bleed speed");
    assert!(vel.length() > 0.0, "drag is a decay, not a brake to zero in 2s");
}

#[test]
fn turn_input_maps_to_angular_velocity() {
    let (_, ccw) = integrate_ship(Vec2::ZERO, 0.0, 1.0, false, DT);
    let (_, cw) = integrate_ship(Vec2::ZERO, 0.0, -1.0, false, DT);
    let (_, idle) = integrate_ship(Vec2::ZERO, 0.0, 0.0, false, DT);
    assert_eq!(ccw, SHIP_TURN_SPEED);
    assert_eq!(cw, -SHIP_TURN_SPEED);
    assert_eq!(idle, 0.0);
}

#[test]
fn can_fire_respects_cooldown_and_live_cap() {
    assert!(can_fire(0.0, 0));
    assert!(!can_fire(0.1, 0), "cooling down");
    assert!(!can_fire(0.0, MAX_LIVE_BULLETS), "at the live-bullet cap");
    assert!(can_fire(0.0, MAX_LIVE_BULLETS - 1));
}

#[test]
fn invincibility_halved_only_in_insane_family() {
    assert_eq!(invincibility_secs(ChaosMode::Normal), RESPAWN_INVINCIBILITY);
    assert_eq!(invincibility_secs(ChaosMode::Ridiculous), RESPAWN_INVINCIBILITY);
    let halved = RESPAWN_INVINCIBILITY * INSANE_INVINCIBILITY_MULT;
    assert_eq!(invincibility_secs(ChaosMode::Insane), halved);
    assert_eq!(invincibility_secs(ChaosMode::Insiculous), halved);
}

#[test]
fn blink_shows_when_vulnerable_and_toggles_during_iframes() {
    assert!(blink_visible(0.0));
    assert!(blink_visible(-1.0));
    // Adjacent half-periods of the blink square wave disagree.
    let half_period = 1.0 / INVINCIBILITY_BLINK_HZ;
    assert_ne!(blink_visible(0.5 * half_period), blink_visible(1.5 * half_period));
}

// --- Splitting and waves ---

#[test]
fn split_spec_matrix_across_modes() {
    for mode in [ChaosMode::Normal, ChaosMode::Insane] {
        assert_eq!(split_spec(AsteroidSize::Large, mode), Some((AsteroidSize::Medium, 2)));
    }
    for mode in [ChaosMode::Ridiculous, ChaosMode::Insiculous] {
        assert_eq!(
            split_spec(AsteroidSize::Large, mode),
            Some((AsteroidSize::Medium, 3)),
            "{mode:?} must split large rocks three ways"
        );
    }
    for mode in ChaosMode::ALL {
        assert_eq!(split_spec(AsteroidSize::Medium, mode), Some((AsteroidSize::Small, 2)));
        assert_eq!(split_spec(AsteroidSize::Small, mode), None);
    }
}

#[test]
fn split_velocity_is_deterministic_and_children_diverge() {
    let parent = Vec2::new(80.0, 0.0);
    let a = split_velocity(parent, 0, 2, 42);
    let b = split_velocity(parent, 1, 2, 43);
    assert_eq!(a, split_velocity(parent, 0, 2, 42), "same seed, same result");
    let angle_between = a.angle_to(b).abs();
    assert!(angle_between > 0.3, "children must fan out, got {angle_between} rad");
    assert!(a.length() > parent.length(), "children fly faster than the parent");
}

#[test]
fn asteroid_polygon_is_deterministic_and_varies_by_seed() {
    let a = asteroid_polygon(7, 40.0);
    assert_eq!(a, asteroid_polygon(7, 40.0));
    assert_ne!(a, asteroid_polygon(8, 40.0), "different seeds, different rocks");
}

#[test]
fn asteroid_polygon_vert_count_and_radius_bounds() {
    let radius = ASTEROID_RADII[0];
    let poly = asteroid_polygon(123, radius);
    assert_eq!(poly.len(), ASTEROID_VERTS);
    for v in &poly {
        let r = v.length();
        assert!(r >= radius * 0.72 - 0.001 && r <= radius * 1.18 + 0.001, "vertex radius {r}");
    }
}

#[test]
fn ship_hull_is_closed_with_nose_along_heading() {
    let pos = Vec2::new(10.0, -5.0);
    let rot = 0.7;
    let segs = ship_segments(pos, rot);
    // Closed loop: each segment ends where the next begins.
    for i in 0..segs.len() {
        assert_eq!(segs[i].1, segs[(i + 1) % segs.len()].0);
    }
    // The nose sits SHIP_SIZE along the heading.
    let nose = segs[0].0;
    let expected = pos + Vec2::new(rot.cos(), rot.sin()) * SHIP_SIZE;
    assert!((nose - expected).length() < 0.001);
}

#[test]
fn wave_count_grows_caps_and_splits_chaos_axes() {
    // Growth and cap.
    assert_eq!(wave_asteroid_count(1, ChaosMode::Normal), WAVE_BASE_ASTEROIDS);
    assert_eq!(wave_asteroid_count(2, ChaosMode::Normal), WAVE_BASE_ASTEROIDS + 1);
    assert_eq!(wave_asteroid_count(99, ChaosMode::Normal), WAVE_MAX_ASTEROIDS);
    // Insane buffs speed, not count; Ridiculous buffs count, not speed.
    assert_eq!(wave_asteroid_count(1, ChaosMode::Insane), WAVE_BASE_ASTEROIDS);
    assert_eq!(
        wave_asteroid_count(1, ChaosMode::Ridiculous),
        WAVE_BASE_ASTEROIDS + RIDICULOUS_EXTRA_ASTEROIDS
    );
    assert_eq!(
        wave_asteroid_count(1, ChaosMode::Insiculous),
        WAVE_BASE_ASTEROIDS + RIDICULOUS_EXTRA_ASTEROIDS
    );
}

#[test]
fn asteroid_speed_stays_in_range_and_insane_multiplies() {
    for size in [AsteroidSize::Large, AsteroidSize::Medium, AsteroidSize::Small] {
        let (min, max) = size.speed_range();
        for seed in 0..50 {
            let normal = asteroid_speed(size, seed, ChaosMode::Normal);
            assert!(normal >= min && normal <= max, "{size:?} speed {normal} outside [{min},{max}]");
            let insane = asteroid_speed(size, seed, ChaosMode::Insane);
            assert!((insane - normal * INSANE_SPEED_MULT).abs() < 0.001);
            assert_eq!(asteroid_speed(size, seed, ChaosMode::Ridiculous), normal);
        }
    }
}

#[test]
fn spawn_position_respects_safe_distance_for_adversarial_ships() {
    // Center, near every border midpoint, and near a corner.
    let ships = [
        Vec2::ZERO,
        Vec2::new(0.0, HALF.y - 10.0),
        Vec2::new(0.0, -(HALF.y - 10.0)),
        Vec2::new(HALF.x - 10.0, 0.0),
        Vec2::new(-(HALF.x - 10.0), 0.0),
        Vec2::new(HALF.x - 5.0, HALF.y - 5.0),
    ];
    for ship in ships {
        for seed in 0..200 {
            let pos = spawn_position(seed, ship);
            assert!(
                pos.distance(ship) >= SAFE_SPAWN_DIST,
                "seed {seed}: spawn {pos:?} within safe distance of ship {ship:?}"
            );
            assert!(
                pos.x.abs() <= HALF.x + 0.001 && pos.y.abs() <= HALF.y + 0.001,
                "spawn {pos:?} must sit on the window border"
            );
        }
    }
    // Deterministic.
    assert_eq!(spawn_position(9, Vec2::ZERO), spawn_position(9, Vec2::ZERO));
}

#[test]
fn size_tables_are_monotonic() {
    const { assert!(ASTEROID_RADII[0] > ASTEROID_RADII[1] && ASTEROID_RADII[1] > ASTEROID_RADII[2]) };
    // Smaller rocks are harder targets, so they pay more.
    const { assert!(ASTEROID_SCORES[0] < ASTEROID_SCORES[1] && ASTEROID_SCORES[1] < ASTEROID_SCORES[2]) };
}

#[test]
fn extra_life_grants_exactly_once_per_threshold() {
    let mut game = AsteroidsGame::default();
    let mut mgr = AchievementManager::in_memory();
    crate::achievements::register_all(&mut mgr);
    let lives = game.lives;

    game.award_points(&mut mgr, EXTRA_LIFE_EVERY - 20);
    assert_eq!(game.lives, lives, "below the threshold: no bonus ship");
    game.award_points(&mut mgr, 20);
    assert_eq!(game.lives, lives + 1, "crossing the threshold grants one ship");
    game.award_points(&mut mgr, 100);
    assert_eq!(game.lives, lives + 1, "the same threshold never pays twice");
    game.award_points(&mut mgr, EXTRA_LIFE_EVERY * 2);
    assert_eq!(game.lives, lives + 3, "a big jump pays every threshold it crosses");
    assert!(mgr.is_unlocked(crate::achievements::SCORE_10K));
}

#[test]
fn circles_overlap_matches_radius_sum() {
    assert!(circles_overlap(Vec2::ZERO, 10.0, Vec2::new(15.0, 0.0), 6.0));
    assert!(!circles_overlap(Vec2::ZERO, 10.0, Vec2::new(17.0, 0.0), 6.0));
}

#[test]
fn closed_polygon_emits_a_vertex_pair_per_edge() {
    let mut lines = Vec::new();
    let square = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];
    push_closed_polygon(&mut lines, &square, Vec2::ZERO, 0.0, Vec4::ONE, 1.0);
    assert_eq!(lines.len(), 8, "4 edges = 8 line-list vertices");
}

#[test]
fn every_mode_has_a_hint() {
    for mode in ChaosMode::ALL {
        assert!(!mode_hint(mode).is_empty(), "{mode:?} needs a mode-select hint");
    }
}

// --- Physics simulations (exact game entity recipes) ---

/// A bullet (dynamic sensor) must report a started contact against a solid
/// dynamic rock — the pair every hit in the game hangs off.
#[test]
fn bullet_registers_hit_on_dynamic_asteroid() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();

    let rock = game.spawn_asteroid(
        &mut world, AsteroidSize::Large, Vec2::new(0.0, 150.0), Vec2::ZERO, 0.0);
    game.spawn_bullet(&mut world, Vec2::ZERO, Vec2::new(0.0, BULLET_SPEED));
    let bullet = game.bullets[0];

    let mut hit = false;
    for _ in 0..120 {
        game.physics.update(&mut world, DT);
        let events = game.physics.take_collision_events();
        if events.iter().any(|c| c.event.started && c.event.involves(bullet, rock)) {
            hit = true;
            break;
        }
    }
    assert!(hit, "bullet never registered a contact with the asteroid");
}

/// Two solid rocks on a collision course must bounce off each other
/// (elastic restitution) — their approach velocities reverse.
#[test]
fn asteroids_bounce_off_each_other() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();

    let a = game.spawn_asteroid(
        &mut world, AsteroidSize::Medium, Vec2::new(-120.0, 0.0), Vec2::new(100.0, 0.0), 0.0);
    let b = game.spawn_asteroid(
        &mut world, AsteroidSize::Medium, Vec2::new(120.0, 0.0), Vec2::new(-100.0, 0.0), 0.0);

    let mut bounced = false;
    for _ in 0..240 {
        game.physics.update(&mut world, DT);
        let va = game.physics.get_body_velocity(a).map(|(v, _)| v).unwrap_or(Vec2::ZERO);
        let vb = game.physics.get_body_velocity(b).map(|(v, _)| v).unwrap_or(Vec2::ZERO);
        if va.x < -1.0 && vb.x > 1.0 {
            bounced = true;
            break;
        }
    }
    assert!(bounced, "solid rocks on a collision course must bounce apart");
}

/// The `Lifetime` safety net must despawn a stray bullet entirely.
#[test]
fn stray_bullet_expires_via_lifetime_system() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();

    game.spawn_bullet(&mut world, Vec2::ZERO, Vec2::new(0.0, BULLET_SPEED));
    let bullet = game.bullets[0];
    assert!(world.get::<Lifetime>(bullet).is_some(), "bullets must carry a Lifetime");

    let frames = (BULLET_LIFETIME * 60.0) as usize + 10;
    for _ in 0..frames {
        game.physics.update(&mut world, DT);
        game.lifetimes.update(&mut world, DT);
    }
    assert!(
        !world.entities().contains(&bullet),
        "expired bullet must be removed from the world"
    );
}

/// GPP-09 wrap proof: writing `Transform2D.position` on a live dynamic body
/// teleports it in rapier AND preserves its velocity — the mechanism every
/// screen wrap in the game relies on.
#[test]
fn transform_write_teleports_live_body_preserving_velocity() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();

    let vel = Vec2::new(120.0, 30.0);
    let rock = game.spawn_asteroid(
        &mut world, AsteroidSize::Medium, Vec2::new(300.0, 0.0), vel, 0.5);
    for _ in 0..10 {
        game.physics.update(&mut world, DT);
    }
    let (vel_before, spin_before) =
        game.physics.get_body_velocity(rock).expect("rock body must exist");

    // Teleport to the far side, exactly what wrap_entities does.
    let target = Vec2::new(-380.0, 0.0);
    world.get_mut::<Transform2D>(rock).expect("rock has a transform").position = target;
    game.physics.update(&mut world, DT);

    let pos = world.get::<Transform2D>(rock).expect("rock has a transform").position;
    assert!(
        (pos - target).length() < 10.0,
        "body must teleport to the written position, ended at {pos:?}"
    );
    let (vel_after, spin_after) =
        game.physics.get_body_velocity(rock).expect("rock body must exist");
    assert!((vel_after - vel_before).length() < 1.0, "teleport must preserve linear velocity");
    assert!((spin_after - spin_before).abs() < 0.05, "teleport must preserve angular velocity");
}

/// `spawn_wave` places the right number of large rocks, all outside the
/// safe radius around the ship.
#[test]
fn spawn_wave_respects_count_and_safe_distance() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();
    let ship = spawn_ship(&mut world);
    game.ship = Some(ship);

    game.wave = 3;
    game.chaos_mode = ChaosMode::Ridiculous;
    game.spawn_wave(&mut world, Vec2::ZERO);

    assert_eq!(game.asteroids.len(), wave_asteroid_count(3, ChaosMode::Ridiculous));
    for rock in &game.asteroids {
        let pos = world.get::<Transform2D>(rock.entity).expect("rock has a transform").position;
        assert!(
            pos.distance(Vec2::ZERO) >= SAFE_SPAWN_DIST,
            "rock at {pos:?} spawned inside the safe radius"
        );
        assert_eq!(rock.size, AsteroidSize::Large);
    }
}

/// The full flight model wired to a real body: thrust accelerates the ship
/// in rapier, and the split constant sanity holds (spread is under a
/// half-circle so children always keep some forward momentum).
#[test]
fn thrust_moves_the_real_ship_body() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();
    let ship = spawn_ship(&mut world);
    game.ship = Some(ship);

    for _ in 0..60 {
        let (vel, _) = game.physics.get_body_velocity(ship).unwrap_or((Vec2::ZERO, 0.0));
        let rot = world.get::<Transform2D>(ship).map(|t| t.rotation).unwrap_or(FRAC_PI_2);
        let (new_vel, angular) = integrate_ship(vel, rot, 0.0, true, DT);
        game.physics.set_velocity(ship, new_vel, angular);
        game.physics.update(&mut world, DT);
    }
    let pos = world.get::<Transform2D>(ship).expect("ship has a transform").position;
    assert!(pos.y > 20.0, "one second of thrust must move the ship up, got {pos:?}");
    // Split fan must stay under a half-circle so children keep momentum.
    const { assert!(SPLIT_SPREAD * 2.0 < PI) };
}
