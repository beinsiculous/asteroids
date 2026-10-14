use engine_core::prelude::*;

use crate::art::Pilot;
use crate::constants::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ModeSelect { selection: u8 },
    Achievements,
    Playing,
    /// Endless game: the only ending is running out of ships.
    GameOver,
}

/// How many ships the human(s) fly. Co-op shares one rock field but keeps
/// per-ship lives and scores; the match only ends once every ship is out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameMode {
    SinglePlayer,
    TwoPlayerCoop,
}

impl GameMode {
    /// Number of ships (and local players) this mode flies.
    pub(crate) fn player_count(self) -> usize {
        match self {
            GameMode::SinglePlayer => 1,
            GameMode::TwoPlayerCoop => 2,
        }
    }
}

/// One flyable ship and everything private to it. In co-op each ship keeps
/// its own lives, score, bullets, and i-frames; `entity` becomes `None` once
/// the ship is out of lives (its body is despawned, the survivor plays on).
pub(crate) struct ShipState {
    pub(crate) entity: Option<EntityId>,
    /// Where the ship points, radians counterclockwise from east, kept in [0, 2π). The
    /// body's rotation is locked: the sprite shows the nearest of the sixteen drawn
    /// headings, and thrust and fire follow that drawn heading, never this raw angle.
    pub(crate) heading: f32,
    /// This ship's live bullets — attribution (score, streak) follows the
    /// owning ship, and the live-bullet cap is per ship.
    pub(crate) bullets: Vec<EntityId>,
    pub(crate) fire_cooldown: f32,
    /// Seconds of i-frames left; > 0 means blinking and unhittable.
    pub(crate) invincibility: f32,
    /// True while this ship's thrust is held; draws its exhaust.
    pub(crate) thrusting: bool,
    pub(crate) lives: u32,
    pub(crate) score: u32,
    /// Next score threshold that grants this ship a bonus life.
    pub(crate) next_extra_life_at: u32,
    /// Consecutive bullet hits without a miss (sharpshooter tracking).
    pub(crate) shot_streak: u32,
}

impl Default for ShipState {
    fn default() -> Self {
        Self {
            entity: None,
            heading: std::f32::consts::FRAC_PI_2,
            bullets: Vec::new(),
            fire_cooldown: 0.0,
            invincibility: 0.0,
            thrusting: false,
            lives: STARTING_LIVES,
            score: 0,
            next_extra_life_at: EXTRA_LIFE_EVERY,
            shot_streak: 0,
        }
    }
}

impl ShipState {
    /// A fresh ship bound to a live body.
    pub(crate) fn new(entity: EntityId) -> Self {
        Self {
            entity: Some(entity),
            ..Self::default()
        }
    }
}

/// Size class of a rock. Index order matches the constant tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl AsteroidSize {
    pub(crate) fn idx(self) -> usize {
        match self {
            AsteroidSize::Large => 0,
            AsteroidSize::Medium => 1,
            AsteroidSize::Small => 2,
        }
    }

    /// The round body's radius, world pixels: what is drawn and what collides. The arms
    /// and fists reach past it and are drawn only.
    pub(crate) fn radius(self) -> f32 {
        ASTEROID_RADII[self.idx()]
    }

    pub(crate) fn score(self) -> u32 {
        ASTEROID_SCORES[self.idx()]
    }

    pub(crate) fn speed_range(self) -> (f32, f32) {
        ASTEROID_SPEED_RANGES[self.idx()]
    }
}

/// A live meatieroid: its body and its size.
pub(crate) struct Asteroid {
    pub(crate) entity: EntityId,
    pub(crate) size: AsteroidSize,
}

/// Every loaded sheet. Placeholders until `init` loads the synced art.
pub(crate) struct Sheets {
    pub(crate) flying_funk: SpriteSheet,
    pub(crate) flying_funk_player_two: SpriteSheet,
    pub(crate) meatieroid_large: SpriteSheet,
    pub(crate) meatieroid_medium: SpriteSheet,
    pub(crate) meatieroid_small: SpriteSheet,
    pub(crate) icicle: SpriteSheet,
    pub(crate) ice_chip: SpriteSheet,
    pub(crate) space: SpriteSheet,
}

impl Sheets {
    pub(crate) fn funk(&self, pilot: Pilot) -> &SpriteSheet {
        match pilot {
            Pilot::Deion => &self.flying_funk,
            Pilot::Cubert => &self.flying_funk_player_two,
        }
    }

    pub(crate) fn shot(&self, pilot: Pilot) -> &SpriteSheet {
        match pilot {
            Pilot::Deion => &self.icicle,
            Pilot::Cubert => &self.ice_chip,
        }
    }

    pub(crate) fn meatieroid(&self, size: AsteroidSize) -> &SpriteSheet {
        match size {
            AsteroidSize::Large => &self.meatieroid_large,
            AsteroidSize::Medium => &self.meatieroid_medium,
            AsteroidSize::Small => &self.meatieroid_small,
        }
    }
}

impl Default for Sheets {
    fn default() -> Self {
        Self {
            flying_funk: SpriteSheet::placeholder(),
            flying_funk_player_two: SpriteSheet::placeholder(),
            meatieroid_large: SpriteSheet::placeholder(),
            meatieroid_medium: SpriteSheet::placeholder(),
            meatieroid_small: SpriteSheet::placeholder(),
            icicle: SpriteSheet::placeholder(),
            ice_chip: SpriteSheet::placeholder(),
            space: SpriteSheet::placeholder(),
        }
    }
}

pub struct AsteroidsGame {
    /// Zero-gravity rapier world; the ship, rocks, and bullets all live here.
    pub(crate) physics: PhysicsSystem,

    /// The flyable ships: one in single player, two in co-op. They share the
    /// rock field but keep individual lives, scores, and bullets.
    pub(crate) ships: Vec<ShipState>,
    pub(crate) mode: GameMode,
    pub(crate) asteroids: Vec<Asteroid>,
    pub(crate) sheets: Sheets,
    /// The star tiles, under everything; they stay up under the menus.
    pub(crate) space: Option<EntityId>,
    /// The deforming grid the engine simulates and draws between the night and the stars.
    pub(crate) backdrop: Option<EntityId>,
    /// Detached one-shots still playing — a meatieroid's `die`, a Funk's `crash`. Each ends
    /// itself with its clip and is forgotten the next frame; the list is how a restart or a
    /// quit ends the ones still playing.
    pub(crate) transient_visuals: Vec<EntityId>,
    /// White 1x1 texture the particle bursts draw with.
    pub(crate) tex_id: u32,

    pub(crate) wave: u32,

    /// Breather countdown between clearing a wave and the next one.
    pub(crate) wave_clear_timer: f32,
    /// Monotonic seed stream for every hash-based decision (shapes, speeds,
    /// spins, spawn points) — the game's only randomness source.
    pub(crate) spawn_counter: u32,

    /// Waves survived since the last death (untouchable tracking). Reset when
    /// ANY ship dies.
    pub(crate) waves_without_death: u32,
    /// Game-clock seconds at the previous kill (double-tap tracking; shared,
    /// so either player's kills can chain).
    pub(crate) last_kill_time: f32,
    /// Game-clock seconds since the match started.
    pub(crate) play_time: f32,

    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Shared pause menu (Resume / Restart / Quit to Title). Only Playing is
    /// pausable; see the pause gate in `update_gameplay`.
    pub(crate) pause: PauseMenu,

    /// F1 toggles magenta collider outlines over the art.
    pub(crate) debug_colliders: bool,
}

impl Default for AsteroidsGame {
    fn default() -> Self {
        Self {
            physics: PhysicsSystem::with_config(PhysicsConfig::space()),
            ships: Vec::new(),
            mode: GameMode::SinglePlayer,
            asteroids: Vec::new(),
            sheets: Sheets::default(),
            space: None,
            backdrop: None,
            transient_visuals: Vec::new(),
            tex_id: 0,
            wave: 1,
            wave_clear_timer: 0.0,
            spawn_counter: 0,
            waves_without_death: 0,
            last_kill_time: f32::NEG_INFINITY,
            play_time: 0.0,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            pause: PauseMenu::new(),
            debug_colliders: false,
        }
    }
}
