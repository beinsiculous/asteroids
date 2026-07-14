use engine_core::prelude::*;

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

    /// Nominal (mean polygon) radius, world pixels.
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

/// A live rock: its body plus the hash-seeded local-space polygon generated
/// once at spawn (stable outline, no per-frame hashing).
pub(crate) struct Asteroid {
    pub(crate) entity: EntityId,
    pub(crate) size: AsteroidSize,
    /// Local-space outline vertices, already radius-scaled.
    pub(crate) shape: Vec<Vec2>,
}

pub(crate) struct AsteroidsGame {
    /// Zero-gravity rapier world; the ship, rocks, and bullets all live here.
    pub(crate) physics: PhysicsSystem,
    /// Despawns expired bullets automatically.
    pub(crate) lifetimes: LifetimeSystem,

    pub(crate) ship: Option<EntityId>,
    pub(crate) asteroids: Vec<Asteroid>,
    pub(crate) bullets: Vec<EntityId>,
    pub(crate) background: Option<EntityId>,
    /// White 1x1 texture backing every sprite (bullets, particles, backdrop).
    pub(crate) tex_id: u32,

    pub(crate) score: u32,
    pub(crate) lives: u32,
    pub(crate) wave: u32,
    /// Next score threshold that grants a bonus ship.
    pub(crate) next_extra_life_at: u32,

    pub(crate) fire_cooldown: f32,
    /// Seconds of i-frames left; > 0 means blinking and unhittable.
    pub(crate) invincibility: f32,
    /// Breather countdown between clearing a wave and the next one.
    pub(crate) wave_clear_timer: f32,
    /// Monotonic seed stream for every hash-based decision (shapes, speeds,
    /// spins, spawn points) — the game's only randomness source.
    pub(crate) spawn_counter: u32,
    /// True while thrust is held; drives the flame and effects.
    pub(crate) thrusting: bool,

    /// Consecutive bullet hits without a miss (sharpshooter tracking).
    pub(crate) shot_streak: u32,
    /// Waves survived since the last death (untouchable tracking).
    pub(crate) waves_without_death: u32,
    /// Game-clock seconds at the previous kill (double-tap tracking).
    pub(crate) last_kill_time: f32,
    /// Game-clock seconds since the match started.
    pub(crate) play_time: f32,

    pub(crate) state: GameState,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    /// Deforming spring-mass grid drawn under the wireframes.
    pub(crate) grid: Option<GridMesh>,
    /// F1 toggles magenta collider outlines over the wireframes.
    pub(crate) debug_colliders: bool,
}

impl Default for AsteroidsGame {
    fn default() -> Self {
        Self {
            physics: PhysicsSystem::with_config(PhysicsConfig::space()),
            lifetimes: LifetimeSystem::new(),
            ship: None,
            asteroids: Vec::new(),
            bullets: Vec::new(),
            background: None,
            tex_id: 0,
            score: 0,
            lives: STARTING_LIVES,
            wave: 1,
            next_extra_life_at: EXTRA_LIFE_EVERY,
            fire_cooldown: 0.0,
            invincibility: 0.0,
            wave_clear_timer: 0.0,
            spawn_counter: 0,
            thrusting: false,
            shot_streak: 0,
            waves_without_death: 0,
            last_kill_time: f32::NEG_INFINITY,
            play_time: 0.0,
            state: GameState::TitleScreen { selection: 0 },
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            grid: None,
            debug_colliders: false,
        }
    }
}
