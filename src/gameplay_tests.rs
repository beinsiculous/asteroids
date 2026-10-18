//! Behavioral tests: pure rules (flight model, the drawn heading, wrapping, splitting,
//! waves) and headless physics simulations proving every rapier pair the game depends
//! on, using the exact spawn recipes.

use engine_core::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::art::*;
use crate::constants::*;
use crate::gameplay::*;
use crate::menu::mode_hint;
use crate::spawning::{child_machine, ship_spawn_pose, shot_rotation, spawn_ship, wave_machine};
use crate::types::*;

/// A ship body as `spawn_ships` builds it, on placeholder sheets: the physics tests need
/// only its body and collider.
fn ship_at(world: &mut World, position: Vec2) -> EntityId {
    spawn_ship(world, &Sheets::default(), Pilot::Deion, position)
}

/// A fabricated started contact between two entities, for pairing tests.
fn started_contact(a: EntityId, b: EntityId) -> CollisionData {
    CollisionData {
        event: CollisionEvent { entity_a: a, entity_b: b, started: true, stopped: false },
        contacts: Vec::new(),
    }
}

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
fn thrust_accelerates_along_the_drawn_heading() {
    let vel = integrate_ship(Vec2::ZERO, heading_direction(0.0), true, DT);
    assert!(vel.x > 0.0, "heading 0 = +x, thrust must push +x");
    assert!(vel.y.abs() < 1e-4);

    let vel = integrate_ship(Vec2::ZERO, heading_direction(FRAC_PI_2), true, DT);
    assert!(vel.y > 0.0, "heading pi/2 = +y, thrust must push +y");
    assert!(vel.x.abs() < 1e-3);

    // Just short of halfway to the next drawn heading, thrust still goes straight east:
    // the ship goes where it is drawn pointing, not along the raw heading.
    let vel = integrate_ship(Vec2::ZERO, heading_direction(0.19), true, DT);
    assert!(vel.y.abs() < 1e-4, "heading 0.19 rad draws heading 00 and thrusts along it, got {vel:?}");
    // Past halfway it is drawn at 22.5 degrees, and thrusts along that.
    let vel = integrate_ship(Vec2::ZERO, heading_direction(0.21), true, DT);
    let bearing = vel.y.atan2(vel.x);
    assert!((bearing - TAU / 16.0).abs() < 1e-4, "heading 0.21 rad thrusts along 22.5 degrees, got {bearing}");
}

#[test]
fn speed_clamps_at_max() {
    let mut vel = Vec2::ZERO;
    for _ in 0..3000 {
        vel = integrate_ship(vel, heading_direction(0.3), true, DT);
    }
    assert!(vel.length() <= SHIP_MAX_SPEED + 0.001, "speed {} over cap", vel.length());
    assert!(vel.length() > SHIP_MAX_SPEED * 0.95, "thrust should reach the cap");
}

#[test]
fn drag_decays_coasting_velocity_without_reversing() {
    let mut vel = Vec2::new(200.0, 0.0);
    let start = vel.length();
    for _ in 0..120 {
        vel = integrate_ship(vel, heading_direction(0.0), false, DT);
        assert!(vel.x >= 0.0, "drag must never reverse the drift direction");
    }
    assert!(vel.length() < start, "coasting must bleed speed");
    assert!(vel.length() > 0.0, "drag is a decay, not a brake to zero in 2s");
}

#[test]
fn turn_input_advances_the_heading_and_keeps_it_in_range() {
    let start = 1.0;
    assert!((advance_heading(start, 1.0, DT) - (start + SHIP_TURN_SPEED * DT)).abs() < 1e-6, "+1 turns counterclockwise");
    assert!((advance_heading(start, -1.0, DT) - (start - SHIP_TURN_SPEED * DT)).abs() < 1e-6, "-1 turns clockwise");
    assert_eq!(advance_heading(start, 0.0, DT), start);

    // A right turn from east wraps to just under a full turn instead of going negative.
    let wrapped = advance_heading(0.0, -1.0, DT);
    assert!((0.0..TAU).contains(&wrapped), "heading {wrapped} left [0, 2pi)");
    assert!((wrapped - (TAU - SHIP_TURN_SPEED * DT)).abs() < 1e-5);
    // A left turn past a full turn wraps to just past east.
    let wrapped = advance_heading(TAU - 0.01, 1.0, DT);
    assert!((0.0..TAU).contains(&wrapped));
    // A turn that lands a hair below zero never rounds up to exactly 2pi.
    assert!(advance_heading(1e-9, -1e-6, 1.0) < TAU);
}

// --- The drawn heading ---

#[test]
fn every_heading_draws_its_own_clips() {
    for index in 0..HEADINGS {
        let heading = index as f32 * TAU / HEADINGS as f32;
        assert_eq!(displayed_heading(heading), index);
        assert_eq!(ship_pose(heading, false, 0.0).clip, IDLE_HEADINGS[index]);
        assert_eq!(ship_pose(heading, true, 0.0).clip, THRUST_HEADINGS[index]);
    }
    assert_eq!(IDLE_HEADINGS[4], "idle_04", "the spawn heading, pi/2, is the fifth drawn");
    assert_eq!(displayed_heading(FRAC_PI_2), 4);
}

#[test]
fn negative_and_full_turn_headings_draw_a_real_heading() {
    // A heading is kept in [0, 2pi), but the display never trusts it: a negative or
    // full-turn angle still names one of the sixteen.
    assert_eq!(displayed_heading(-0.01), 0);
    assert_eq!(displayed_heading(-TAU / 16.0), 15);
    assert_eq!(displayed_heading(TAU - 0.01), 0, "a hair under a full turn rounds to east");
    assert_eq!(displayed_heading(TAU), 0);
    assert_eq!(displayed_heading(TAU + TAU / 16.0), 1);
    assert_eq!(ship_pose(-0.3, false, 0.0).clip, "idle_15");
}

#[test]
fn turning_across_a_heading_boundary_keeps_the_frame() {
    // The frame comes from the match clock, so turning through a heading lands on the
    // frame the blink is on rather than restarting the clip.
    let boundary = TAU / 32.0;
    for play_time in [0.0, 0.1, 0.2, 0.33, 1.7] {
        for thrusting in [false, true] {
            let before = ship_pose(boundary - 1e-4, thrusting, play_time);
            let after = ship_pose(boundary + 1e-4, thrusting, play_time);
            assert_ne!(before.clip, after.clip, "the boundary changes the drawn heading");
            assert_eq!(before.frame, after.frame, "at {play_time}s the frame survives the turn");
        }
    }
    // Both clips step through their two frames at the art's rate.
    assert_eq!(ship_pose(0.0, false, 0.0).frame, 0);
    assert_eq!(ship_pose(0.0, false, IDLE_FRAME_SECONDS + 0.01).frame, 1);
    assert_eq!(ship_pose(0.0, false, 2.0 * IDLE_FRAME_SECONDS + 0.01).frame, 0);
    assert_eq!(ship_pose(0.0, true, THRUST_FRAME_SECONDS + 0.01).frame, 1);
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
            let pos = spawn_position(seed, &[ship]);
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
    assert_eq!(spawn_position(9, &[Vec2::ZERO]), spawn_position(9, &[Vec2::ZERO]));
}

/// Co-op waves must keep clear of BOTH ships at once, not just one.
#[test]
fn wave_rocks_spawn_clear_of_both_ships() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();
    game.mode = GameMode::TwoPlayerCoop;

    let (p0, f0) = ship_spawn_pose(2, 0);
    let (p1, f1) = ship_spawn_pose(2, 1);
    game.ships.push(ShipState { heading: f0, ..ShipState::new(ship_at(&mut world, p0)) });
    game.ships.push(ShipState { heading: f1, ..ShipState::new(ship_at(&mut world, p1)) });

    game.wave = 4;
    game.spawn_wave(&mut world);

    assert_eq!(game.asteroids.len(), wave_asteroid_count(4, ChaosMode::Normal));
    for rock in &game.asteroids {
        let pos = world.get::<Transform2D>(rock.entity).expect("rock has a transform").position;
        assert!(pos.distance(p0) >= SAFE_SPAWN_DIST, "rock {pos:?} too close to P1 at {p0:?}");
        assert!(pos.distance(p1) >= SAFE_SPAWN_DIST, "rock {pos:?} too close to P2 at {p1:?}");
    }
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
    game.ships.push(ShipState::default());
    let mut mgr = AchievementManager::in_memory();
    crate::achievements::register_all(&mut mgr);
    let lives = game.ships[0].lives;

    game.award_points(&mut mgr, 0, EXTRA_LIFE_EVERY - 20);
    assert_eq!(game.ships[0].lives, lives, "below the threshold: no bonus ship");
    game.award_points(&mut mgr, 0, 20);
    assert_eq!(game.ships[0].lives, lives + 1, "crossing the threshold grants one ship");
    game.award_points(&mut mgr, 0, 100);
    assert_eq!(game.ships[0].lives, lives + 1, "the same threshold never pays twice");
    game.award_points(&mut mgr, 0, EXTRA_LIFE_EVERY * 2);
    assert_eq!(game.ships[0].lives, lives + 3, "a big jump pays every threshold it crosses");
    assert!(mgr.is_unlocked(crate::achievements::SCORE_10K));
}

/// Each ship banks its own points and crosses its own extra-life thresholds;
/// one player earning a bonus ship never touches the other's lives.
#[test]
fn extra_life_thresholds_pay_per_player() {
    let mut game = AsteroidsGame::default();
    game.ships.push(ShipState::default());
    game.ships.push(ShipState::default());
    let mut mgr = AchievementManager::in_memory();
    crate::achievements::register_all(&mut mgr);
    let base = STARTING_LIVES;

    // P1 crosses a threshold; P2 does not.
    game.award_points(&mut mgr, 0, EXTRA_LIFE_EVERY);
    assert_eq!(game.ships[0].lives, base + 1, "P1 earns a bonus ship");
    assert_eq!(game.ships[1].lives, base, "P2's lives are untouched by P1's score");

    // P2 earns its own, independently.
    game.award_points(&mut mgr, 1, EXTRA_LIFE_EVERY);
    assert_eq!(game.ships[1].lives, base + 1, "P2 earns its own bonus ship");
    assert_eq!(game.ships[0].lives, base + 1, "P1 unchanged by P2's score");
}

/// The match ends only when every ship is out of lives; a single survivor
/// keeps it going. In solo the slice has one element.
#[test]
fn coop_game_over_requires_both_ships_out() {
    assert!(!coop_over(&[1, 0]), "P1 still has a life");
    assert!(!coop_over(&[0, 2]), "P2 still has lives");
    assert!(coop_over(&[0, 0]), "both out = game over");
    assert!(coop_over(&[0]), "solo: out of lives = game over");
    assert!(!coop_over(&[3]), "solo: lives remain");
}

/// Solo spawns the lone ship at the center; co-op spawns two disjoint poses.
#[test]
fn ship_spawn_poses_are_disjoint_in_coop_and_centered_solo() {
    let (solo_pos, solo_facing) = ship_spawn_pose(1, 0);
    assert_eq!(solo_pos, Vec2::ZERO, "single player starts at the center");
    assert_eq!(solo_facing, FRAC_PI_2, "facing up");

    let (a, fa) = ship_spawn_pose(2, 0);
    let (b, fb) = ship_spawn_pose(2, 1);
    assert_ne!(a, b, "co-op ships must not share a spawn point");
    assert!(a.distance(b) > 1.0, "co-op ships spawn well apart");
    assert_eq!(fa, FRAC_PI_2);
    assert_eq!(fb, FRAC_PI_2);
}

/// Structural no-friendly-fire guard: `resolve_hit_pairs`' only hittable set
/// is the rocks slice, so a started contact between a bullet and a ship never
/// resolves as a hit — while a bullet-vs-rock contact does.
#[test]
fn bullets_never_test_against_ships() {
    let mut world = World::new();
    let other_ship = ship_at(&mut world, Vec2::new(80.0, 0.0));
    let bullet = world.spawn().id();
    let rock = world.spawn().id();

    let bullets = [(0usize, bullet)];
    let rocks = [rock];

    let vs_ship = started_contact(bullet, other_ship);
    assert!(
        resolve_hit_pairs(&bullets, &rocks, &[vs_ship]).is_empty(),
        "a bullet touching a ship must never resolve as a hit"
    );

    let vs_rock = started_contact(bullet, rock);
    assert_eq!(
        resolve_hit_pairs(&bullets, &rocks, &[vs_rock]),
        vec![(0, bullet, rock)],
        "a bullet touching a rock resolves and is attributed to its owner"
    );
}

#[test]
fn circles_overlap_matches_radius_sum() {
    assert!(circles_overlap(Vec2::ZERO, 10.0, Vec2::new(15.0, 0.0), 6.0));
    assert!(!circles_overlap(Vec2::ZERO, 10.0, Vec2::new(17.0, 0.0), 6.0));
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

    game.ships.push(ShipState::default());
    let rock = game.spawn_asteroid(
        &mut world, AsteroidSize::Large, Vec2::new(0.0, 150.0), Vec2::ZERO, 0.0, wave_machine());
    game.spawn_bullet(&mut world, 0, Vec2::ZERO, Vec2::new(0.0, BULLET_SPEED));
    let bullet = game.ships[0].bullets[0];

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
        &mut world, AsteroidSize::Medium, Vec2::new(-120.0, 0.0), Vec2::new(100.0, 0.0), 0.0, wave_machine());
    let b = game.spawn_asteroid(
        &mut world, AsteroidSize::Medium, Vec2::new(120.0, 0.0), Vec2::new(-100.0, 0.0), 0.0, wave_machine());

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
    let mut lifetimes = LifetimeSystem::new();

    game.ships.push(ShipState::default());
    game.spawn_bullet(&mut world, 0, Vec2::ZERO, Vec2::new(0.0, BULLET_SPEED));
    let bullet = game.ships[0].bullets[0];
    assert!(world.get::<Lifetime>(bullet).is_some(), "bullets must carry a Lifetime");

    let frames = (BULLET_LIFETIME * 60.0) as usize + 10;
    for _ in 0..frames {
        game.physics.update(&mut world, DT);
        lifetimes.update(&mut world, DT);
    }
    assert!(
        !world.entities().contains(&bullet),
        "expired bullet must be removed from the world"
    );
}

/// Screen-wrap proof: writing `Transform2D.position` on a live dynamic body
/// teleports it in rapier AND preserves its velocity — the mechanism every
/// screen wrap in the game relies on.
#[test]
fn transform_write_teleports_live_body_preserving_velocity() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();

    let vel = Vec2::new(120.0, 30.0);
    let rock = game.spawn_asteroid(
        &mut world, AsteroidSize::Medium, Vec2::new(300.0, 0.0), vel, 0.5, wave_machine());
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
    let ship = ship_at(&mut world, Vec2::ZERO);
    game.ships.push(ShipState::new(ship));

    game.wave = 3;
    game.chaos_mode = ChaosMode::Ridiculous;
    game.spawn_wave(&mut world);

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

/// The full flight model wired to a real body: thrust along the drawn heading moves
/// the ship in rapier while its locked body never turns, and the split constant sanity
/// holds (spread is under a half-circle so children always keep some forward momentum).
#[test]
fn thrust_moves_the_real_ship_body_along_its_heading() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();
    let ship = ship_at(&mut world, Vec2::ZERO);
    game.ships.push(ShipState::new(ship));
    let heading = 3.0 * TAU / 16.0;

    for _ in 0..60 {
        let (vel, _) = game.physics.get_body_velocity(ship).unwrap_or((Vec2::ZERO, 0.0));
        let new_vel = integrate_ship(vel, heading_direction(heading), true, DT);
        game.physics.set_velocity(ship, new_vel, 0.0);
        game.physics.update(&mut world, DT);
    }
    let transform = world.get::<Transform2D>(ship).expect("ship has a transform");
    assert!(transform.position.length() > 20.0, "one second of thrust must move the ship, got {:?}", transform.position);
    let bearing = transform.position.y.atan2(transform.position.x);
    assert!((bearing - heading).abs() < 1e-3, "the ship flew along {bearing}, not its heading {heading}");
    assert_eq!(transform.rotation, 0.0, "the body never turns");
    // Split fan must stay under a half-circle so children keep momentum.
    const { assert!(SPLIT_SPREAD * 2.0 < PI) };
}

// --- The drawn sizes ---

/// The body radii, the sprite offsets and the wrap margins are the measured numbers
/// (`art_tests.rs` re-measures them from the pixels).
#[test]
fn the_size_table_is_the_measured_bodies() {
    assert_eq!(ASTEROID_RADII, [29.5, 18.5, 11.5], "the drawn bodies' radii");
    for size in [AsteroidSize::Large, AsteroidSize::Medium, AsteroidSize::Small] {
        let sheet = size.art().sheet;
        let (top_left, bottom_right) = sheet.bounds;
        assert_eq!(bottom_right - top_left, Vec2::splat(2.0 * size.radius()), "{size:?}'s box is its body circle's");
        assert_eq!(sheet.sprite_offset(), Vec2::new(-0.5, 1.5), "{size:?}'s body centre sits on the entity");
        assert!(sheet.is_within_cell(), "{size:?}");
        assert!(MEATIEROID_REACH[size.idx()] > size.radius(), "{size:?} wraps by its fist, past its body");
    }
    assert_eq!(MEATIEROID_REACH, [54.0, 30.0, 18.0]);
    assert_eq!(SHIP_COLLIDER_RADIUS, 11.0);
    assert_eq!(MUZZLE_OFFSET, FUNK_SPOUT_REACH + 4.0, "a shot leaves just past the spout");
}

// --- Splitting ---

/// Every child spawned by one split, in fan order.
fn split_once(chaos: ChaosMode, size: AsteroidSize, parent_vel: Vec2, start: u32) -> Vec<(Vec2, Vec2, AsteroidSize)> {
    let mut game = AsteroidsGame { chaos_mode: chaos, spawn_counter: start, ..AsteroidsGame::default() };
    let mut world = World::new();
    game.spawn_children(&mut world, size, Vec2::ZERO, parent_vel);
    let born: Vec<(EntityId, Vec2, AsteroidSize)> = game.asteroids.iter()
        .map(|rock| {
            let position = world.get::<Transform2D>(rock.entity).expect("a child has a transform").position;
            (rock.entity, position, rock.size)
        })
        .collect();
    // The launch velocities are buffered until the bodies first sync.
    game.physics.update(&mut world, 1e-6);
    born.into_iter()
        .map(|(entity, position, size)| {
            let (velocity, _) = game.physics.get_body_velocity(entity).expect("a child has a body");
            (position, velocity, size)
        })
        .collect()
}

/// The split's jitter for a seed, as `split_velocity` draws it.
fn jitter_for(seed: u32) -> f32 {
    (hash_f32(seed) - 0.5) * 0.3
}

/// Starts for the seed stream that swing two neighbouring children toward each other
/// hardest: `spawn_children` draws one seed per child and `spawn_asteroid` one more, so
/// child `i` jitters by `start + 1 + 2i`. Picked from a million starts; the stream's
/// hash never puts two such neighbours at the ±0.15 rad extreme together, and these are
/// the closest it comes (about 0.115 rad each, inward).
fn inward_extreme_starts(count: usize) -> Vec<u32> {
    let mut best: Vec<(f32, u32)> = (0..1_000_000u32)
        .map(|start| {
            let swing = (0..count - 1)
                .map(|left| jitter_for(start + 1 + 2 * left as u32).min(-jitter_for(start + 3 + 2 * left as u32)))
                .fold(f32::MIN, f32::max);
            (swing, start)
        })
        .collect();
    best.sort_by(|a, b| b.0.total_cmp(&a.0));
    best.into_iter().take(20).map(|(_, start)| start).collect()
}

/// A split's children are born clear of each other through the real spawn path, for
/// both the two-way split and the Ridiculous three-way. Placed along the unjittered fan,
/// a child is born at the same spot whatever its velocity's jitter — so starts that
/// swing two neighbours toward each other hardest, which once closed the fan and
/// overlapped the siblings, cannot move them.
#[test]
fn split_children_are_born_clear_of_each_other() {
    let parent_vel = Vec2::new(60.0, 25.0);
    let base = parent_vel.y.atan2(parent_vel.x);
    for (chaos, size) in [
        (ChaosMode::Normal, AsteroidSize::Large),
        (ChaosMode::Ridiculous, AsteroidSize::Large),
        (ChaosMode::Normal, AsteroidSize::Medium),
    ] {
        let count = split_spec(size, chaos).expect("a large or medium splits").1;
        let extremes = inward_extreme_starts(count);
        let birthplaces: Vec<Vec2> = split_once(chaos, size, parent_vel, 0).into_iter().map(|(position, _, _)| position).collect();
        for start in extremes.iter().copied().chain(0..500) {
            let children = split_once(chaos, size, parent_vel, start);
            assert_eq!(children.len(), count);
            for (index, &(position, velocity, _)) in children.iter().enumerate() {
                assert!(position.distance(birthplaces[index]) < 1e-3, "{chaos:?} {size:?} start {start}: child {index} moved with its jitter");
                let fan = base + (index as f32 - (count as f32 - 1.0) / 2.0) * SPLIT_SPREAD;
                let jitter = (velocity.y.atan2(velocity.x) - fan + PI).rem_euclid(TAU) - PI;
                assert!(jitter.abs() <= 0.15 + 1e-4, "the velocity keeps the split's own jitter");
            }
            for index in 0..count - 1 {
                let (left, _, left_size) = children[index];
                let (right, _, right_size) = children[index + 1];
                let gap = left.distance(right) - left_size.radius() - right_size.radius();
                assert!(gap > 1.5, "{chaos:?} {size:?} start {start}: siblings {index} and {} clear by only {gap}", index + 1);
            }
        }
        let swing = extremes.iter()
            .map(|&start| (0..count - 1).map(|left| jitter_for(start + 1 + 2 * left as u32).min(-jitter_for(start + 3 + 2 * left as u32))).fold(f32::MIN, f32::max))
            .fold(f32::MIN, f32::max);
        assert!(swing > 0.11, "{chaos:?} {size:?}: the chosen starts swing a pair inward by {swing} rad at best");
    }
}

#[test]
fn split_offsets_follow_the_unjittered_fan() {
    let parent_vel = Vec2::new(0.0, 80.0);
    let large = split_offset(parent_vel, AsteroidSize::Large, AsteroidSize::Medium, 0, 2);
    assert!((large.length() - 44.53).abs() < 0.01, "a large's children sit 44.5 px out, got {}", large.length());
    let medium = split_offset(parent_vel, AsteroidSize::Medium, AsteroidSize::Small, 1, 2);
    assert!((medium.length() - 28.44).abs() < 0.01, "a medium's children sit 28.4 px out, got {}", medium.length());
    let bearing = large.y.atan2(large.x);
    assert!((bearing - (FRAC_PI_2 - SPLIT_SPREAD / 2.0)).abs() < 1e-5, "child 0 sits half a spread clockwise of the parent's heading");
}

/// A split's children are born flexing: `attack`, then `idle`.
#[test]
fn split_children_run_the_flex_machine() {
    let machine = child_machine();
    assert_eq!(machine.state(), ATTACK);
    assert_eq!(wave_machine().state(), IDLE);
}

// --- Shots ---

/// Each shot is turned once, at spawn, to its flight: the drawn icicle points up, so its
/// turn is the velocity's bearing less a quarter turn. Its locked body flies straight, so
/// physics never sees the rotation change, and no shot in flight is ever pushed back
/// into rapier as an outside edit — which would teleport it.
#[test]
fn shots_are_turned_to_their_flight_and_never_pushed() {
    let mut game = AsteroidsGame::default();
    let mut world = World::new();
    game.ships.push(ShipState::default());
    game.ships.push(ShipState::default());
    for index in 0..8 {
        let angle = index as f32 * TAU / 8.0 + 0.1;
        let direction = Vec2::new(angle.cos(), angle.sin());
        game.spawn_bullet(&mut world, index % 2, direction * 30.0, direction * BULLET_SPEED);
    }
    let shots: Vec<EntityId> = game.ships.iter().flat_map(|ship| ship.bullets.iter().copied()).collect();
    for frame in 0..20 {
        game.physics.update(&mut world, DT);
        assert_eq!(game.physics.external_edits_pushed_last_update(), 0, "frame {frame}: a shot was pushed as an edit");
        for &shot in &shots {
            let rotation = world.get::<Transform2D>(shot).expect("a shot has a transform").rotation;
            let (velocity, _) = game.physics.get_body_velocity(shot).expect("a shot has a body");
            let off = (rotation - shot_rotation(velocity) + PI).rem_euclid(TAU) - PI;
            assert!(off.abs() < 1e-4, "frame {frame}: a shot drawn {off} rad off its flight");
        }
    }
    assert!((shot_rotation(Vec2::Y) - 0.0).abs() < 1e-6, "a shot flying up is drawn as painted");
    assert!((shot_rotation(Vec2::X) + FRAC_PI_2).abs() < 1e-6, "a shot flying east turns a quarter clockwise");
}

/// A co-op ship out of lives still scores the shots it left in flight, but a crossing
/// never gives it a life back — one with no body to fly would keep the match from ever
/// ending once the other ship goes down.
#[test]
fn an_eliminated_ship_scores_but_never_earns_a_life_back() {
    let mut game = AsteroidsGame::default();
    game.ships.push(ShipState { lives: 0, ..ShipState::default() });
    game.ships.push(ShipState { lives: 1, ..ShipState::default() });
    let mut mgr = AchievementManager::in_memory();
    crate::achievements::register_all(&mut mgr);

    game.award_points(&mut mgr, 0, EXTRA_LIFE_EVERY);
    assert_eq!(game.ships[0].score, EXTRA_LIFE_EVERY, "the parting shot still scores");
    assert_eq!(game.ships[0].lives, 0, "an eliminated ship stays out");
    game.ships[1].lives = 0;
    let lives: Vec<u32> = game.ships.iter().map(|ship| ship.lives).collect();
    assert!(coop_over(&lives), "the match ends when the last flying ship goes down");
}
