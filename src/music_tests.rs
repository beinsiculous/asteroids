//! Meatieroids' music through the real game and the engine's harness, whose audio manager plays
//! the committed song on its virtual clock: the sidecar, the bank and the score as committed, the
//! seven moments and their sources, the order within a frame, the pause jam, the grid's pulse, a
//! match start deciding by the song in the slot, and a missing bank leaving the game silent.

use std::path::PathBuf;
use std::time::Duration;

use engine_core::prelude::*;
use engine_core::test_support::{GameHarness, InputEvent};
use music_midi::rustysynth::SoundFont;
use music_midi::{BackgroundScore, InstrumentSource, MusicalUnit, PartMode, PositionSnapshot, Quantize, SongData, SongFile};

use crate::art::FUNK_REACH;
use crate::constants::{EXTRA_LIFE_EVERY, WAVE_CLEAR_DELAY, WIN_W};
use crate::music::SONG_PATH;
use crate::test_support::*;
use crate::types::*;

/// What a trigger waits for before the conductor has drained it: far longer than it takes.
const DRAINED: Duration = Duration::from_secs(2);
/// Ticks in one bar of 5/4 at 480 a quarter.
const BAR_TICKS: u64 = 2400;

fn committed_song() -> SongData {
    let bytes = std::fs::read(asset_base().join(SONG_PATH)).expect("the song is committed");
    SongData::from_smf(&bytes).expect("the song parses").0
}

fn committed_sidecar() -> SongFile {
    let text = std::fs::read_to_string(asset_base().join(format!("{SONG_PATH}.ron"))).expect("the sidecar");
    music_midi::parse_song_file("ai_meatieroids.mid.ron", &text).expect("the sidecar parses")
}

fn units_of<'song>(song: &'song SongData, track: &str) -> &'song [MusicalUnit] {
    song.tracks().iter().find(|candidate| candidate.name() == Some(track)).expect(track).units()
}

fn conductor(harness: &GameHarness<AsteroidsGame>) -> PositionSnapshot {
    harness.audio().song_snapshot().expect("a song is in the slot").conductor
}

#[test]
fn the_committed_sidecar_binds_nine_moments_on_seven_unquantized_parts() {
    let file = committed_sidecar();
    let binding = |action: &str| file.bindings.iter().find(|binding| binding.action == action).expect(action).clone();
    let mode_of = |action: &str| file.parts.iter().find(|part| part.name == binding(action).part).expect(action).mode;
    let mut parts: Vec<&str> = file.parts.iter().map(|part| part.name.as_str()).collect();
    parts.sort_unstable();
    assert_eq!(parts, ["crash", "drum", "fanfare", "jam", "life", "surge", "theme"], "the background tracks select no part");
    assert_eq!(file.bindings.len(), 9, "no hyperspace: the game has none");
    for (action, instrument) in [("large", "tuba"), ("medium", "trombone"), ("small", "trumpet")] {
        assert_eq!((binding(action).part.as_str(), binding(action).instrument.as_str()), ("theme", instrument), "{action}");
    }
    assert_eq!(mode_of("shot"), PartMode::AtSongPosition, "a shot drums the beat of its moment");
    for action in ["large", "thrust", "crash", "wave", "life", "menu"] {
        assert_eq!(mode_of(action), PartMode::Loop, "{action}");
    }
    assert!(file.parts.iter().all(|part| part.quantize == Quantize::Off), "no trigger is parked on the loop seam");
    assert!(file.provenance.is_some());
    assert_eq!(file.soundfont, "soundfonts/meatieroids.sf2");
    let song = committed_song();
    for cue in ["surge", "crash", "fanfare", "life"] {
        assert_eq!(units_of(&song, cue).len(), 1, "a cue is one note or one chord: Loop plays one unit a trigger ({cue})");
    }
}

/// Whether `key` sounds on `preset` of `bank`: a preset region and one of its instrument's
/// regions both cover it (a region without a key range covers every key).
fn bank_covers(bank: &SoundFont, preset: u8, key: u8) -> bool {
    let key = i32::from(key);
    let preset = bank.get_presets().iter()
        .find(|candidate| candidate.get_bank_number() == 0 && candidate.get_patch_number() == i32::from(preset))
        .expect("the preset is in the bank");
    preset.get_regions().iter()
        .filter(|region| region.get_key_range_start() <= key && key <= region.get_key_range_end())
        .any(|region| bank.get_instruments()[region.get_instrument_id()].get_regions().iter()
            .any(|inner| inner.get_key_range_start() <= key && key <= inner.get_key_range_end()))
}

#[test]
fn every_key_of_the_shared_theme_sounds_on_each_of_its_three_instruments() {
    let file = committed_sidecar();
    let bytes = std::fs::read(asset_base().join(&file.soundfont)).expect("the bank is committed");
    let bank = SoundFont::new(&mut bytes.as_slice()).expect("the bank parses");
    let song = committed_song();
    let keys: Vec<u8> = units_of(&song, "theme").iter().flat_map(|unit| unit.notes.iter().map(|note| note.key)).collect();
    assert!((24..=32).contains(&keys.len()), "the theme is 24 to 32 units");
    for action in ["large", "medium", "small"] {
        let binding = file.bindings.iter().find(|binding| binding.action == action).expect(action);
        let instrument = file.instruments.iter().find(|instrument| instrument.name == binding.instrument).expect("bound");
        let InstrumentSource::SoundFont { preset, .. } = instrument.source else { panic!("{action} is a SoundFont preset") };
        for &key in &keys {
            assert!(bank_covers(&bank, preset, key), "key {key} of the theme is silent on {}'s preset {preset}", binding.instrument);
        }
    }
}

#[test]
fn the_song_is_thirty_two_bars_of_five_four_at_160_and_its_drum_and_ostinato_keep_the_bars() {
    let song = committed_song();
    let meter = song.time_signatures().first().expect("a meter is written, not the 4/4 default");
    assert_eq!((meter.tick, meter.numerator, meter.denominator), (0, 5, 4), "else the pulse's bars drift off Mars");
    let loop_end = BackgroundScore::new(&song, &Default::default()).end_tick();
    assert_eq!(loop_end, 32 * BAR_TICKS, "the tuba's held pedal ends it on a downbeat");
    let tempo = song.tempo_map();
    assert_eq!(tempo.microseconds_per_quarter_at(0), 375_000, "160 BPM from the first tick");
    assert!((tempo.seconds_at_tick(loop_end) - 60.0).abs() < 1e-9, "the loop is sixty seconds");

    let end_of = |unit: &MusicalUnit| unit.tick + unit.notes.iter().map(|note| note.duration_ticks).max().unwrap_or(0);
    let drum = units_of(&song, "drum");
    assert_eq!(drum[0].tick, 0, "the drum line starts on the first tick");
    for pair in drum.windows(2) {
        assert_eq!(end_of(&pair[0]), pair[1].tick, "a unit ends where the next begins, at {}", pair[1].tick);
    }
    assert_eq!(end_of(drum.last().expect("a unit")), loop_end, "the last ends on the loop's end, so no shot lands in a rest");

    let ostinato = units_of(&song, "ostinato");
    for bar in 0..32 {
        let starts: Vec<u64> = ostinato.iter().map(|unit| unit.tick).filter(|tick| tick / BAR_TICKS == bar).map(|tick| tick % BAR_TICKS).collect();
        assert_eq!(starts, [0, 160, 320, 480, 960, 1440, 1680, 1920], "bar {}: a triplet, two quarters, two eighths, a quarter", bar + 1);
    }
}

#[test]
fn a_shot_drums_once_on_its_ships_source_and_a_press_under_the_cooldown_or_the_cap_sends_nothing() {
    let (mut harness, _) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    let funk = ship(&harness, 0);
    let before = conductor(&harness);
    step_with(&mut harness, &[press(KeyCode::Space)]);
    assert_eq!(harness.game().music.triggered, ["shot"]);
    assert!(source_of(&harness, funk).is_some(), "the ship's own activation");
    step_with(&mut harness, &[release(KeyCode::Space)]);
    step_with(&mut harness, &[press(KeyCode::Space)]);
    assert!(harness.game().music.triggered.is_empty(), "a press under the cooldown fires nothing and sends nothing");
    step_with(&mut harness, &[release(KeyCode::Space)]);
    for _ in 0..3 {
        harness.context(|game, _| game.ships[0].fire_cooldown = 0.0);
        key(&mut harness, KeyCode::Space);
    }
    assert_eq!(harness.game().ships[0].bullets.len(), 4, "four shots live: the cap");
    harness.context(|game, _| game.ships[0].fire_cooldown = 0.0);
    step_with(&mut harness, &[press(KeyCode::Space)]);
    assert!(harness.game().music.triggered.is_empty(), "a press at the cap sends nothing");
    assert_eq!(harness.game().music.counts.shots, 4);
    hold_the_match(&mut harness);
    poll(&mut harness, DRAINED, "four accepted drums", |harness| conductor(harness).accepted_triggers == before.accepted_triggers + 4);
    assert_eq!(conductor(&harness).silent_triggers, before.silent_triggers, "the drum line had a note sounding");
}

#[test]
fn each_size_broken_walks_the_theme_on_its_own_binding_on_the_games_source() {
    let (mut harness, anchor) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    let (before, mut traces) = (conductor(&harness), Vec::new());
    for size in [AsteroidSize::Large, AsteroidSize::Medium, AsteroidSize::Small] {
        let rock = place_meatieroid(&mut harness, size, Vec2::new(200.0, 150.0));
        shoot_at(&mut harness, 0, rock);
        traces.extend(next_trace(&mut harness));
        assert!(!alive(&harness, rock));
        assert_eq!(source_of(&harness, rock), None, "a meatieroid is never a source");
        clear_but(&mut harness, anchor);
    }
    assert_eq!(traces, ["large", "medium", "small"], "each size to its own binding, the big ones low");
    assert_eq!(harness.game().music.counts.broken, [1, 1, 1]);
    hold_the_match(&mut harness);
    let rung = |harness: &GameHarness<AsteroidsGame>| {
        let now = conductor(harness);
        now.accepted_triggers == before.accepted_triggers + 3 && now.live_sources == before.live_sources + 1
    };
    poll(&mut harness, DRAINED, "three accepted on the game's one source", rung);
    settle(&mut harness);
    assert!(rung(&harness), "the breaks ring on the game's source, which no despawn ends");
}

#[test]
fn thrust_sounds_on_its_rising_edge_only_and_before_a_shot_on_the_same_frame() {
    let (mut harness, _) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    let funk = ship(&harness, 0);
    step_with(&mut harness, &[press(KeyCode::KeyW)]);
    assert_eq!(harness.game().music.triggered, ["thrust"]);
    assert!(source_of(&harness, funk).is_some(), "on the ship's own source");
    for _ in 0..30 {
        harness.step(FRAME, &[]);
        assert!(harness.game().music.triggered.is_empty(), "holding thrust sends nothing more");
    }
    assert!(harness.game().ships[0].thrusting);
    step_with(&mut harness, &[release(KeyCode::KeyW)]);
    assert!(harness.game().music.triggered.is_empty(), "releasing sends nothing");
    step_with(&mut harness, &[press(KeyCode::KeyW)]);
    assert_eq!(harness.game().music.triggered, ["thrust"]);
    assert_eq!(harness.game().music.counts.thrusts, 2);
    step_with(&mut harness, &[release(KeyCode::KeyW)]);
    step_with(&mut harness, &[press(KeyCode::KeyW), press(KeyCode::Space)]);
    assert_eq!(harness.game().music.triggered, ["thrust", "shot"], "on one frame, a ship's thrust before its shot");
}

#[test]
fn a_surviving_crash_keeps_the_ships_source_and_the_last_life_ends_it() {
    let (mut harness, anchor) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    let funk = ship(&harness, 0);
    key(&mut harness, KeyCode::Space);
    assert!(source_of(&harness, funk).is_some(), "a shot activated the ship's source");
    let spot = position_of(&harness, funk);
    place_meatieroid(&mut harness, AsteroidSize::Small, spot);
    assert_eq!(next_trace(&mut harness), ["crash"]);
    assert_eq!(harness.game().music.counts.crashes, 1);
    assert!(source_of(&harness, funk).is_some(), "a surviving ship is teleported and keeps its source");
    clear_but(&mut harness, anchor);
    harness.context(|game, _| {
        game.ships[0].lives = 1;
        game.ships[0].invincibility = 0.0;
    });
    let spot = position_of(&harness, funk);
    place_meatieroid(&mut harness, AsteroidSize::Small, spot);
    assert_eq!(next_trace(&mut harness), ["crash"]);
    assert_eq!(harness.game().music.counts.crashes, 2);
    assert_eq!(harness.game().state, GameState::GameOver);
    assert_eq!(source_of(&harness, funk), None, "the ship went with its last life, its source with it");
}

#[test]
fn an_extra_life_sounds_inside_the_break_that_earned_it() {
    let (mut harness, _) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    harness.context(|game, _| game.ships[0].score = EXTRA_LIFE_EVERY - AsteroidSize::Large.score());
    let lives = harness.game().ships[0].lives;
    let rock = place_meatieroid(&mut harness, AsteroidSize::Large, Vec2::new(200.0, 150.0));
    shoot_at(&mut harness, 0, rock);
    assert_eq!(next_trace(&mut harness), ["large", "life"]);
    assert_eq!(harness.game().ships[0].lives, lives + 1);
    assert_eq!(harness.game().music.counts.lives, 1);
}

#[test]
fn the_wave_fanfare_lands_on_the_spawn_edge_and_the_clear_edge_sends_nothing() {
    let mut harness = empty_field(GameMode::SinglePlayer, ChaosMode::Normal);
    let rock = place_meatieroid(&mut harness, AsteroidSize::Small, Vec2::new(200.0, 150.0));
    shoot_at(&mut harness, 0, rock);
    assert_eq!(next_trace(&mut harness), ["small"], "the last break sends its note alone: the breath is the signal");
    let breath = (WAVE_CLEAR_DELAY / FRAME).ceil() as usize + 2;
    let mut frames = 0;
    loop {
        harness.step(FRAME, &[]);
        frames += 1;
        assert!(frames <= breath, "the next wave within its breath");
        if harness.game().music.triggered == ["wave"] {
            break;
        }
        assert!(harness.game().music.triggered.is_empty() && harness.game().asteroids.is_empty(), "nothing sent while the field is clear");
    }
    assert!(!harness.game().asteroids.is_empty(), "the fanfare lands with the rocks");
    assert!(frames + 2 >= breath, "two seconds of frames later, not on the clear edge");
    assert_eq!(harness.game().music.counts.waves, 1);
}

#[test]
fn two_shots_breaking_two_meatieroids_on_one_frame_trigger_in_the_hit_pairs_order() {
    let (mut harness, _) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    let large = place_meatieroid(&mut harness, AsteroidSize::Large, Vec2::new(-150.0, 150.0));
    let medium = place_meatieroid(&mut harness, AsteroidSize::Medium, Vec2::new(150.0, 150.0));
    let spots = [position_of(&harness, large), position_of(&harness, medium)];
    shoot_at(&mut harness, 0, large);
    shoot_at(&mut harness, 0, medium);
    let one_shots = harness.game().transient_visuals.len();
    let trace = next_trace(&mut harness);
    let dies = &harness.game().transient_visuals[one_shots..];
    assert_eq!(dies.len(), 2, "both broke on the one frame");
    let broken_in_order: Vec<&str> = dies.iter().map(|&die| {
        let at = position_of(&harness, die);
        if at.distance(spots[0]) < 1.0 { "large" } else { assert!(at.distance(spots[1]) < 1.0); "medium" }
    }).collect();
    assert_eq!(trace, broken_in_order, "each break triggers as its pair resolves, one large and one medium");
}

#[test]
fn the_pause_menu_jams_one_unit_a_move_and_nothing_on_open_confirm_the_pointer_or_a_reopen() {
    let (mut harness, _) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    let jams = |harness: &GameHarness<AsteroidsGame>| harness.game().music.game.jam_units();
    harness.step(FRAME, &[press(KeyCode::Escape), press(KeyCode::KeyW)]);
    harness.step(FRAME, &[release(KeyCode::Escape), release(KeyCode::KeyW)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 0, "the frame it opens jams nothing, a move on it included");
    let accepted = conductor(&harness).accepted_triggers;
    key(&mut harness, KeyCode::KeyS);
    assert_eq!(jams(&harness), 1, "one move, one unit");
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1, "the song plays on under the pause menu");
    let (resume, restart) = (drawn_at(&harness, "Resume"), drawn_at(&harness, "Restart"));
    for at in [restart, resume, restart, resume] {
        harness.step(FRAME, &[InputEvent::MouseMoved(at.x, at.y)]);
    }
    assert_eq!(jams(&harness), 1, "a pointer swept across the rows jams nothing");
    harness.step(FRAME, &[InputEvent::MouseMoved(resume.x, resume.y)]);
    key(&mut harness, KeyCode::Enter);
    assert!(!harness.game().pause.is_active(), "confirm on Resume resumes");
    assert_eq!(jams(&harness), 1, "and confirm jams nothing");
    harness.step(FRAME, &[press(KeyCode::Escape), press(KeyCode::KeyS)]);
    harness.step(FRAME, &[release(KeyCode::Escape), release(KeyCode::KeyS)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 1, "a reopen jams nothing on its opening frame");
    assert_eq!(harness.game().music.triggered, Vec::<&str>::new());
}

#[test]
fn a_beat_ripples_under_each_live_ship_where_the_frame_drew_it_after_the_wrap() {
    let (mut harness, anchor) = anchored(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    let (survivor, out) = (ship(&harness, 0), ship(&harness, 1));
    harness.context(|game, _| game.ships[1].lives = 1);
    let spot = position_of(&harness, out);
    place_meatieroid(&mut harness, AsteroidSize::Small, spot);
    assert_eq!(next_trace(&mut harness), ["crash"]);
    assert_eq!(harness.game().ships[1].entity, None, "player two is out; player one plays on");
    assert_eq!(harness.game().state, GameState::Playing);
    clear_but(&mut harness, anchor);
    let past_the_edge = WIN_W / 2.0 + FUNK_REACH + 4.0;
    let (mut beats, mut frames) = (0, 0);
    while beats < 2 {
        frames += 1;
        assert!(frames <= 600, "two beats within six hundred frames of wall time, not a hung suite");
        harness.context(|game, ctx| {
            game.ships[0].invincibility = 1.0;
            ctx.world.get_mut::<Transform2D>(survivor).expect("a transform").position = Vec2::new(past_the_edge, 40.0);
        });
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
        let pulsed_at = harness.game().music.pulsed_at.clone();
        if pulsed_at.is_empty() {
            continue;
        }
        let drawn = position_of(&harness, survivor);
        assert_eq!(pulsed_at, [drawn], "one anchor, the live ship where this frame drew it; the ship out of lives is none");
        assert!(drawn.x < 0.0, "after the wrap, never at the edge it left");
        beats += 1;
    }
}

/// Beats the song has crossed, counted from its top.
fn beats_into_the_song(harness: &GameHarness<AsteroidsGame>) -> u64 {
    harness.audio().song_position().expect("playing").beat
}

#[test]
fn no_beat_ripples_under_the_pause_menu_or_on_the_frame_the_last_life_ends_the_match() {
    let (mut harness, _) = anchored(GameMode::SinglePlayer, ChaosMode::Normal);
    key(&mut harness, KeyCode::Escape);
    assert!(harness.game().pause.is_active());
    let (pulsed, beat) = (harness.game().music.game.pulsed(), beats_into_the_song(&harness));
    poll(&mut harness, Duration::from_secs(3), "two beats crossed under the pause menu", |harness| {
        assert!(harness.game().music.pulsed_at.is_empty(), "nothing rippled under the menu");
        beats_into_the_song(harness) >= beat + 2
    });
    assert_eq!(harness.game().music.game.pulsed(), pulsed, "no beat ripples a frozen grid");
    key(&mut harness, KeyCode::Escape);
    assert!(!harness.game().pause.is_active());

    for attempt in 0.. {
        assert!(attempt < 10, "a beat on the losing frame within ten tries");
        let funk = ship(&harness, 0);
        harness.context(|game, _| {
            game.ships[0].lives = 1;
            game.ships[0].invincibility = 0.0;
        });
        settle(&mut harness);
        let position = harness.audio().song_position().expect("playing");
        let seconds_a_beat = 60.0 / 160.0;
        let next_beat = (position.seconds / seconds_a_beat).floor() * seconds_a_beat + seconds_a_beat;
        std::thread::sleep(Duration::from_secs_f64(next_beat - position.seconds + 0.01));
        let pulsed = harness.game().music.game.pulsed();
        let spot = position_of(&harness, funk);
        place_meatieroid(&mut harness, AsteroidSize::Small, spot);
        harness.step(FRAME, &[]);
        assert_eq!(harness.game().music.triggered, ["crash"]);
        assert_eq!(harness.game().state, GameState::GameOver);
        if harness.audio().beat_crossed(Subdivision::Beat) {
            assert!(harness.game().music.pulsed_at.is_empty(), "the frame that ends the match ripples nothing");
            assert_eq!(harness.game().music.game.pulsed(), pulsed);
            return;
        }
        harness.context(|game, ctx| {
            game.start_game(ctx);
            game.destroy_all_asteroids(ctx.world);
        });
    }
}

#[test]
fn the_song_plays_after_init_and_a_match_start_requests_it_into_an_empty_slot_or_takes_it_to_the_top() {
    let mut harness = title_harness();
    let own = format!("{}/{SONG_PATH}", asset_base().display());
    assert!(!harness.game().music.game.song_failed());
    assert_eq!(harness.audio().song_label(), Some(own.as_str()));
    let before = conductor(&harness).blocks_rendered;
    settle(&mut harness);
    assert!(conductor(&harness).blocks_rendered > before, "the virtual clock renders the song");
    harness.audio_mut().stop_song();
    harness.context(|game, ctx| game.start_game(ctx));
    assert_eq!(harness.audio().song_label(), Some(own.as_str()), "an empty slot is requested into");
    hold_the_match(&mut harness);
    poll(&mut harness, DRAINED, "the song past its first beat", |harness| beats_into_the_song(harness) >= 1);
    harness.context(|game, ctx| game.start_game(ctx));
    hold_the_match(&mut harness);
    settle(&mut harness);
    let position = harness.audio().song_position().expect("playing");
    assert_eq!((position.bar, position.beat), (0, 0), "a match begins at the top of Mars");
}

/// An asset folder holding every directory the game reads but `soundfonts/`, each a link to
/// the repository's own, removed however the test ends.
#[cfg(unix)]
struct AssetsWithoutBank {
    root: PathBuf,
}

#[cfg(unix)]
impl AssetsWithoutBank {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("meatieroids-missing-bank-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let assets = root.join("assets");
        std::fs::create_dir_all(assets.join("ai")).expect("a scratch asset folder");
        for directory in ["fonts", "sprites", "ai/music"] {
            std::os::unix::fs::symlink(asset_base().join(directory), assets.join(directory)).expect("linked");
        }
        Self { root }
    }
}

#[cfg(unix)]
impl Drop for AssetsWithoutBank {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[cfg(unix)]
#[test]
fn without_its_bank_the_game_runs_silent_and_counts_every_trigger_it_refuses() {
    let scratch = AssetsWithoutBank::new();
    let assets = scratch.root.join("assets");
    let config = crate::game_config(assets.to_str().expect("UTF-8"));
    let mut harness = GameHarness::new(AsteroidsGame::default(), config);
    harness.step(FRAME, &[]);
    harness.context(|game, ctx| game.start_game(ctx));
    assert!(harness.game().music.game.song_failed(), "the song failed at init");
    assert_eq!(harness.game().music.game.failures(), 2, "and again at the match start");
    assert_eq!(harness.audio().song_label(), None);
    let refused = harness.sound_sources().refused_triggers();
    step_with(&mut harness, &[press(KeyCode::Space)]);
    assert_eq!(harness.game().music.triggered, ["shot"]);
    assert_eq!(harness.sound_sources().refused_triggers(), refused + 1, "refused, and counted");
    assert_eq!(harness.game().state, GameState::Playing, "and the game runs on");
}
