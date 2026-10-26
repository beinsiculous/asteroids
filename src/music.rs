//! Meatieroids' music: Gustav Holst's *Mars, the Bringer of War*, which the engine's conductor
//! plays as a song.
//!
//! The background — the timpani's ostinato in 5/4 over the tuba's G pedal, the trumpet's calls
//! — plays by itself from `init` on, through the menus and under the pause menu. Seven moments
//! trigger the song's bindings, each on the source [`Music::play`] names for it; a move in the
//! pause menu jams the `jam` counter-line; the grid ripples under each live ship on the song's
//! bars and beats. The rules every game's song shares are the engine's `GameMusic` (its `start`
//! is called on `game`); what is here is the game's moments, counts and pulse. A frame sends
//! each ship's thrust then its shot, then the breaks (and a life inside one), then the crashes,
//! then the wave.

use engine_core::prelude::*;

use crate::gameplay::ripple_grid;
use crate::types::AsteroidSize;

/// The song under the asset base: an agent's first cut, under `ai/music` until a person's
/// pass promotes it.
pub(crate) const SONG_PATH: &str = "ai/music/ai_meatieroids.mid";
/// The song's nine bindings: one per meatieroid size on the one `theme` part, the five other
/// moments, and the pause menu's jam.
pub(crate) const SHOT_BINDING: &str = "shot";
pub(crate) const LARGE_BINDING: &str = "large";
pub(crate) const MEDIUM_BINDING: &str = "medium";
pub(crate) const SMALL_BINDING: &str = "small";
pub(crate) const THRUST_BINDING: &str = "thrust";
pub(crate) const CRASH_BINDING: &str = "crash";
pub(crate) const WAVE_BINDING: &str = "wave";
pub(crate) const LIFE_BINDING: &str = "life";
pub(crate) const MENU_BINDING: &str = "menu";
/// A bar line's ripple under each live ship: a large meatieroid's break, felt and not a death.
pub(crate) const BEAT_PULSE_STRENGTH: f32 = 320.0;
pub(crate) const BEAT_PULSE_RADIUS: f32 = 120.0;
/// A beat that is not a bar line ripples at this share of a bar's; zero pulses on the bars only.
pub(crate) const BEAT_PULSE_FRACTION: f32 = 0.25;

/// A moment the game plays, with the entity whose source it plays on where it has one: a
/// ship's shot, a meatieroid of a size broken, a ship's thrust starting, a ship lost, the
/// next wave arriving, an extra ship earned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Moment {
    Shot(EntityId),
    Break(AsteroidSize),
    Thrust(EntityId),
    Crash,
    Wave,
    Life,
}

/// What the game itself sent, beside the conductor's counters: what a listener's ear is
/// checked against. The counts run for the session; a match start never resets them. The
/// jam and the pulse are counted by `GameMusic`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MusicCounts {
    pub(crate) shots: u64,
    /// Broken meatieroids by `AsteroidSize::idx`: large, medium, small.
    pub(crate) broken: [u64; 3],
    pub(crate) thrusts: u64,
    pub(crate) crashes: u64,
    pub(crate) waves: u64,
    pub(crate) lives: u64,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub(crate) struct Music {
    pub(crate) game: GameMusic,
    pub(crate) counts: MusicCounts,
    /// The bindings this frame triggered, in the order the game sent them: production's
    /// call order recorded for the same-frame order tests, never reconstructed.
    #[cfg(test)]
    pub(crate) triggered: Vec<&'static str>,
    /// Where this frame's beat rippled the grid, one anchor a live ship.
    #[cfg(test)]
    pub(crate) pulsed_at: Vec<Vec2>,
}

impl Default for Music {
    fn default() -> Self {
        Self {
            game: GameMusic::new(SONG_PATH, "meatieroids", "MEATIEROIDS_TELEMETRY"),
            counts: MusicCounts::default(),
            #[cfg(test)]
            triggered: Vec::new(),
            #[cfg(test)]
            pulsed_at: Vec::new(),
        }
    }
}

impl Music {
    /// Trigger `moment`'s binding and count it. A broken meatieroid is gone in that call and a
    /// lost ship's last life takes its body with it, so a break, a crash, a wave and a life
    /// ring on the game's source; a shot's and a thrust's notes end with their ship.
    pub(crate) fn play(&mut self, ctx: &mut GameContext, moment: Moment) {
        let counts = &mut self.counts;
        let (binding, entity, count) = match moment {
            Moment::Shot(ship) => (SHOT_BINDING, Some(ship), &mut counts.shots),
            Moment::Break(size) => (break_binding(size), None, &mut counts.broken[size.idx()]),
            Moment::Thrust(ship) => (THRUST_BINDING, Some(ship), &mut counts.thrusts),
            Moment::Crash => (CRASH_BINDING, None, &mut counts.crashes),
            Moment::Wave => (WAVE_BINDING, None, &mut counts.waves),
            Moment::Life => (LIFE_BINDING, None, &mut counts.lives),
        };
        *count += 1;
        let source = if let Some(entity) = entity { ctx.sources.entity(entity) } else { ctx.sources.game() };
        #[cfg(test)]
        self.triggered.push(binding);
        ctx.sources.trigger(ctx.audio, binding, source);
    }

    /// The pause menu's frame: a move up or down jams one unit of the counter-line.
    pub(crate) fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool) {
        if self.game.follow_pause(ctx, paused, MENU_BINDING) {
            #[cfg(test)]
            self.triggered.push(MENU_BINDING);
        }
    }

    /// A running match's last step, after the wrap: a bar line ripples the grid under each of
    /// `ships` — every live ship where it is drawn this frame — and another beat a quarter as
    /// hard. Never called while paused (the frozen grid discards impulses) or off `Playing`.
    pub(crate) fn pulse(&mut self, ctx: &mut GameContext, ships: &[Vec2]) {
        let strength = match self.game.beat_pulse(ctx) {
            Some(BeatPulse::Bar) => BEAT_PULSE_STRENGTH,
            Some(BeatPulse::Beat) => BEAT_PULSE_STRENGTH * BEAT_PULSE_FRACTION,
            None => return,
        };
        #[cfg(test)]
        self.pulsed_at.extend_from_slice(ships);
        for &ship in ships {
            ripple_grid(ctx.world, ship, strength, BEAT_PULSE_RADIUS);
        }
    }

    /// Every frame, whatever the state: the engine's frame ring, telemetry and refusal
    /// lines, the telemetry ending in the game's counts.
    pub(crate) fn update(&mut self, ctx: &mut GameContext) {
        #[cfg(test)]
        {
            self.triggered.clear();
            self.pulsed_at.clear();
        }
        let (counts, jam_units, pulsed) = (self.counts, self.game.jam_units(), self.game.pulsed());
        let [large, medium, small] = counts.broken;
        self.game.update(ctx, || {
            format!(
                "shots={} broken={} large={large} medium={medium} small={small} thrusts={} crashes={} waves={} lives={} jam_units={jam_units} pulsed={pulsed}",
                counts.shots, large + medium + small, counts.thrusts, counts.crashes, counts.waves, counts.lives
            )
        });
    }
}

/// The binding a broken meatieroid of `size` plays on the shared theme: the big ones low.
fn break_binding(size: AsteroidSize) -> &'static str {
    match size {
        AsteroidSize::Large => LARGE_BINDING,
        AsteroidSize::Medium => MEDIUM_BINDING,
        AsteroidSize::Small => SMALL_BINDING,
    }
}
