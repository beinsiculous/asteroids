//! The art Meatieroids draws: one `SheetSpec` per synced sheet, the clip names the
//! game types (the contract with the sidecars), one draw depth per sheet, and the
//! colours of the bursts and the night.
//!
//! Every box is measured from the synced PNGs; `art_tests.rs` re-measures each one, so a
//! redrawn sheet that moves a body, a pivot or a spout fails there, not on the screen.
//! Re-measure the art, never scale a sprite.

use engine_core::prelude::*;

use crate::types::AsteroidSize;

const FUNK_CELL: Vec2 = Vec2::new(64.0, 64.0);
const SHOT_CELL: Vec2 = Vec2::new(16.0, 16.0);
pub(crate) const SPACE_TILE_PX: f32 = 64.0;

// --- the Flying Funks -----------------------------------------------------------------

/// Deion's Funk from above. The box is the oval hull's at heading 00, centred on the
/// cell: the hull turns about the cell centre in every heading, so the entity's position
/// is the pivot and the sprite needs no offset.
pub(crate) const FLYING_FUNK: SheetSpec = SheetSpec {
    path: "sprites/ai_flying_funk_64x64.png",
    cell: FUNK_CELL,
    bounds: (Vec2::new(19.0, 22.0), Vec2::new(45.0, 42.0)),
};
/// Player 2's Funk: the same mask in other ramps, so the same box.
pub(crate) const FLYING_FUNK_PLAYER_TWO: SheetSpec = SheetSpec {
    path: "sprites/ai_flying_funk_player_two_64x64.png",
    cell: FUNK_CELL,
    bounds: (Vec2::new(19.0, 22.0), Vec2::new(45.0, 42.0)),
};

/// How far the spout's tip reaches from the pivot, in every heading (17.5–17.7 px).
pub(crate) const FUNK_SPOUT_REACH: f32 = 18.0;
/// The farthest opaque pixel from the pivot over every `idle` and `thrust` frame — the
/// thrust plume's tip, 29.3 px.
pub(crate) const FUNK_REACH: f32 = 30.0;

// --- the meatieroids --------------------------------------------------------------------

/// Each size's box is its round body's, not the frame's: the arms and fists swing wider
/// and are drawn, not collided. The body is a circle centred half a pixel right of and a
/// pixel and a half below its cell's centre in every `idle` and `attack` frame.
pub(crate) const MEATIEROID_LARGE: SheetSpec = SheetSpec {
    path: "sprites/ai_meatieroid_large.png",
    cell: Vec2::new(128.0, 96.0),
    bounds: (Vec2::new(35.0, 20.0), Vec2::new(94.0, 79.0)),
};
pub(crate) const MEATIEROID_MEDIUM: SheetSpec = SheetSpec {
    path: "sprites/ai_meatieroid_medium.png",
    cell: Vec2::new(80.0, 64.0),
    bounds: (Vec2::new(22.0, 15.0), Vec2::new(59.0, 52.0)),
};
pub(crate) const MEATIEROID_SMALL: SheetSpec = SheetSpec {
    path: "sprites/ai_meatieroid_small.png",
    cell: Vec2::new(48.0, 48.0),
    bounds: (Vec2::new(13.0, 14.0), Vec2::new(36.0, 37.0)),
};

/// How far each size's farthest opaque pixel — a fist — reaches from its body centre over
/// every `idle` and `attack` frame, to the pixel's far edge, in table order (53.05, 29.61
/// and 17.68 px). A meatieroid wraps only once its fist is off the field.
pub(crate) const MEATIEROID_REACH: [f32; 3] = [54.0, 30.0, 18.0];

// --- the shots, each drawn pointing up ----------------------------------------------------

/// Deion's shot: 8 x 12, its tip up.
pub(crate) const ICICLE: SheetSpec = SheetSpec {
    path: "sprites/ai_icicle_16x16.png",
    cell: SHOT_CELL,
    bounds: (Vec2::new(4.0, 2.0), Vec2::new(12.0, 14.0)),
};
/// Cubert's shot: 8 x 7.
pub(crate) const ICE_CHIP: SheetSpec = SheetSpec {
    path: "sprites/ai_ice_chip_16x16.png",
    cell: SHOT_CELL,
    bounds: (Vec2::new(4.0, 5.0), Vec2::new(12.0, 12.0)),
};
/// Half the icicle's longest side and then some: a shot wraps once it is off the field.
pub(crate) const SHOT_REACH: f32 = 8.0;

// --- the stars ----------------------------------------------------------------------------

/// Salt and pepper on a transparent ground: the night behind them is the clear colour and
/// the grid draws between the two.
pub(crate) const SPACE: SheetSpec = SheetSpec {
    path: "sprites/ai_meatieroids_space_64x64.png",
    cell: Vec2::new(SPACE_TILE_PX, SPACE_TILE_PX),
    bounds: (Vec2::ZERO, Vec2::new(SPACE_TILE_PX, SPACE_TILE_PX)),
};

// --- the clip names the game types --------------------------------------------------------

pub(crate) const IDLE: &str = "idle";
pub(crate) const ATTACK: &str = "attack";
pub(crate) const DIE: &str = "die";
pub(crate) const FLY: &str = "fly";
pub(crate) const CRASH: &str = "crash";

/// The Funk's sixteen drawn headings: heading k points k × 22.5° counterclockwise from
/// east, world y up.
pub(crate) const HEADINGS: usize = 16;
pub(crate) const IDLE_HEADINGS: [&str; HEADINGS] = [
    "idle_00", "idle_01", "idle_02", "idle_03", "idle_04", "idle_05", "idle_06", "idle_07",
    "idle_08", "idle_09", "idle_10", "idle_11", "idle_12", "idle_13", "idle_14", "idle_15",
];
pub(crate) const THRUST_HEADINGS: [&str; HEADINGS] = [
    "thrust_00", "thrust_01", "thrust_02", "thrust_03", "thrust_04", "thrust_05", "thrust_06", "thrust_07",
    "thrust_08", "thrust_09", "thrust_10", "thrust_11", "thrust_12", "thrust_13", "thrust_14", "thrust_15",
];
/// Each heading's clips hold two frames, at these rates.
pub(crate) const HEADING_CLIP_FRAMES: usize = 2;
pub(crate) const IDLE_FRAME_SECONDS: f32 = 0.18;
pub(crate) const THRUST_FRAME_SECONDS: f32 = 0.09;

// --- draw depths --------------------------------------------------------------------------
// One depth per sheet: the renderer draws each sheet's sprites as one batch, ordered by its
// lowest depth, and transparent texels write depth — so a sheet with entities both under
// and over another sheet's would punch holes in it. A one-shot takes its own sheet's depth.
// Every one stays under the particles the engine draws at 0.5.

pub(crate) const SPACE_DEPTH: f32 = -2.0;
pub(crate) const MEATIEROID_LARGE_DEPTH: f32 = -0.50;
pub(crate) const MEATIEROID_MEDIUM_DEPTH: f32 = -0.45;
pub(crate) const MEATIEROID_SMALL_DEPTH: f32 = -0.40;
pub(crate) const ICICLE_DEPTH: f32 = -0.30;
pub(crate) const ICE_CHIP_DEPTH: f32 = -0.25;
pub(crate) const FLYING_FUNK_DEPTH: f32 = -0.20;
pub(crate) const FLYING_FUNK_PLAYER_TWO_DEPTH: f32 = -0.15;

// --- colours ------------------------------------------------------------------------------

/// The night: devil's-food darkest, the style guide's hex.
pub(crate) const NIGHT: u32 = 0x241418;
/// A meatieroid bursting: the meatball's crust brown.
pub(crate) const MEAT_BURST_COLOR: Vec4 = rgb(0x9C6B3C);
/// Deion's Funk going down: its ice.
pub(crate) const ICE_BURST_COLOR: Vec4 = rgb(0x29D1EA);
/// Player 2's Funk going down: its berry.
pub(crate) const BERRY_BURST_COLOR: Vec4 = rgb(0xA66FD4);

/// The night as the renderer's clear colour. The clear lands in the linear HDR target
/// where sprite textures arrive already decoded from sRGB, so the style guide's hex is
/// decoded the same way; passing it raw would draw a brighter night than the guide's.
pub(crate) fn night_clear_color() -> [f32; 4] {
    let decode = |channel: u32| {
        let encoded = channel as f32 / 255.0;
        if encoded <= 0.04045 { encoded / 12.92 } else { ((encoded + 0.055) / 1.055).powf(2.4) }
    };
    [decode((NIGHT >> 16) & 0xFF), decode((NIGHT >> 8) & 0xFF), decode(NIGHT & 0xFF), 1.0]
}

// --- the meatieroid table -----------------------------------------------------------------

/// What one size of meatieroid is drawn with.
pub(crate) struct MeatieroidArt {
    pub(crate) sheet: &'static SheetSpec,
    pub(crate) depth: f32,
}

impl AsteroidSize {
    pub(crate) fn art(self) -> MeatieroidArt {
        match self {
            AsteroidSize::Large => MeatieroidArt { sheet: &MEATIEROID_LARGE, depth: MEATIEROID_LARGE_DEPTH },
            AsteroidSize::Medium => MeatieroidArt { sheet: &MEATIEROID_MEDIUM, depth: MEATIEROID_MEDIUM_DEPTH },
            AsteroidSize::Small => MeatieroidArt { sheet: &MEATIEROID_SMALL, depth: MEATIEROID_SMALL_DEPTH },
        }
    }
}

// --- the pilots ---------------------------------------------------------------------------

/// Who flies a ship: Deion solo and as player 1, Cubert as player 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pilot {
    Deion,
    Cubert,
}

/// What one pilot's Funk is drawn with, the shot it fires, and the colour it bursts.
pub(crate) struct PilotArt {
    pub(crate) sheet: &'static SheetSpec,
    pub(crate) depth: f32,
    pub(crate) shot: &'static SheetSpec,
    pub(crate) shot_depth: f32,
    pub(crate) burst_color: Vec4,
    pub(crate) name: &'static str,
}

const DEION_ART: PilotArt = PilotArt {
    sheet: &FLYING_FUNK, depth: FLYING_FUNK_DEPTH, shot: &ICICLE, shot_depth: ICICLE_DEPTH,
    burst_color: ICE_BURST_COLOR, name: "Flying Funk",
};
const CUBERT_ART: PilotArt = PilotArt {
    sheet: &FLYING_FUNK_PLAYER_TWO, depth: FLYING_FUNK_PLAYER_TWO_DEPTH, shot: &ICE_CHIP,
    shot_depth: ICE_CHIP_DEPTH, burst_color: BERRY_BURST_COLOR, name: "Flying Funk P2",
};

impl Pilot {
    /// Who flies ship `index`.
    pub(crate) fn for_ship(index: usize) -> Pilot {
        if index == 0 { Pilot::Deion } else { Pilot::Cubert }
    }

    pub(crate) fn art(self) -> &'static PilotArt {
        match self {
            Pilot::Deion => &DEION_ART,
            Pilot::Cubert => &CUBERT_ART,
        }
    }
}
