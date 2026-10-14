//! The art contract: every number `art.rs` and `constants.rs` measured from the synced
//! sheets is re-measured here from the PNGs themselves, and every clip the game types is
//! held against its sidecar. A redrawn sheet that moves a body, a pivot, a spout or a
//! fist fails here, not on the screen.

use engine_core::prelude::*;
use image::RgbaImage;

use crate::art::*;
use crate::constants::*;
use crate::test_support::{asset_base, sidecar};
use crate::types::AsteroidSize;

/// The synced PNG behind `spec`, decoded.
fn pixels(spec: &SheetSpec) -> RgbaImage {
    let path = asset_base().join(spec.path);
    image::open(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())).to_rgba8()
}

/// The frames `clip` plays on `spec`'s sheet, from its sidecar.
fn frames(spec: &SheetSpec, clip: &str) -> Vec<u32> {
    let sheet = sidecar(spec).sheet;
    let (_, found) = sheet.clips.iter().find(|(name, _)| name == clip)
        .unwrap_or_else(|| panic!("{} has no clip {clip}", spec.path));
    found.frame_indices.clone()
}

/// Every opaque pixel of `frame`, as its centre in cell pixels, Y down.
fn opaque_pixels(image: &RgbaImage, spec: &SheetSpec, frame: u32) -> Vec<Vec2> {
    let columns = image.width() / spec.cell.x as u32;
    let origin = (frame % columns * spec.cell.x as u32, frame / columns * spec.cell.y as u32);
    let mut found = Vec::new();
    for y in 0..spec.cell.y as u32 {
        for x in 0..spec.cell.x as u32 {
            if image.get_pixel(origin.0 + x, origin.1 + y)[3] != 0 {
                found.push(Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
            }
        }
    }
    found
}

/// The opaque box of `frame`, in cell pixels, bottom-right one past the last pixel.
fn frame_box(image: &RgbaImage, spec: &SheetSpec, frame: u32) -> Option<(Vec2, Vec2)> {
    opaque_pixels(image, spec, frame).into_iter()
        .map(|centre| (centre - 0.5, centre + 0.5))
        .reduce(|(min, max), (low, high)| (min.min(low), max.max(high)))
}

/// How far the farthest corner of any opaque pixel in `frames` lies from `centre`.
fn farthest_reach(image: &RgbaImage, spec: &SheetSpec, frames: &[u32], centre: Vec2) -> f32 {
    let corners = [Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5), Vec2::new(-0.5, 0.5), Vec2::new(0.5, 0.5)];
    frames.iter()
        .flat_map(|&frame| opaque_pixels(image, spec, frame))
        .flat_map(|pixel| corners.map(|corner| (pixel + corner).distance(centre)))
        .fold(0.0, f32::max)
}

const FUNKS: [&SheetSpec; 2] = [&FLYING_FUNK, &FLYING_FUNK_PLAYER_TWO];
const SIZES: [AsteroidSize; 3] = [AsteroidSize::Large, AsteroidSize::Medium, AsteroidSize::Small];

/// Every sheet the game draws, with the clips it types and which of them play once.
fn every_sheet() -> Vec<(&'static SheetSpec, Vec<&'static str>, Vec<&'static str>)> {
    let headings: Vec<&str> = IDLE_HEADINGS.iter().chain(THRUST_HEADINGS.iter()).copied()
        .chain([CRASH]).collect();
    let mut sheets: Vec<(&'static SheetSpec, Vec<&'static str>, Vec<&'static str>)> =
        FUNKS.iter().map(|&spec| (spec, headings.clone(), vec![CRASH])).collect();
    sheets.extend(SIZES.map(|size| (size.art().sheet, vec![IDLE, ATTACK, DIE], vec![ATTACK, DIE])));
    sheets.push((&ICICLE, vec![FLY], vec![]));
    sheets.push((&ICE_CHIP, vec![FLY], vec![]));
    sheets.push((&SPACE, vec![IDLE], vec![]));
    sheets
}

#[test]
fn every_clip_the_game_types_is_declared_and_plays_as_drawn() {
    for (spec, clips, play_once) in every_sheet() {
        assert!(spec.is_within_cell(), "{}", spec.path);
        let sheet = sidecar(spec).sheet;
        let image = pixels(spec);
        assert_eq!(
            (image.width(), image.height()),
            (sheet.grid.cols * spec.cell.x as u32, sheet.grid.rows * spec.cell.y as u32),
            "{}'s cell", spec.path,
        );
        for clip in clips {
            let (_, found) = sheet.clips.iter().find(|(name, _)| name == clip)
                .unwrap_or_else(|| panic!("{} declares no {clip}", spec.path));
            assert_eq!(found.looping, !play_once.contains(&clip), "{}'s {clip} loops wrong", spec.path);
        }
    }
}

#[test]
fn every_drawn_heading_holds_two_frames_at_the_pose_rates() {
    for spec in FUNKS {
        let sheet = sidecar(spec).sheet;
        for (clips, seconds) in [(&IDLE_HEADINGS, IDLE_FRAME_SECONDS), (&THRUST_HEADINGS, THRUST_FRAME_SECONDS)] {
            for clip in clips {
                let (_, found) = sheet.clips.iter().find(|(name, _)| name == clip).expect("declared");
                assert_eq!(found.frame_indices.len(), HEADING_CLIP_FRAMES, "{}'s {clip}", spec.path);
                assert!((found.fps - 1.0 / seconds).abs() < 0.01, "{}'s {clip} runs at {} fps", spec.path, found.fps);
            }
        }
    }
}

/// The Funk turns about its cell's centre: heading k + 8 is heading k turned half a turn,
/// so its opaque box is k's box turned about the pivot. A pivot off the centre by d
/// would move the turned box by 2d.
#[test]
fn both_funks_turn_about_the_cell_centre() {
    for spec in FUNKS {
        assert_eq!(spec.sprite_offset(), Vec2::ZERO, "{}: the hull's box is centred on the cell", spec.path);
        let image = pixels(spec);
        let centre = spec.cell * 0.5;
        for clips in [&IDLE_HEADINGS, &THRUST_HEADINGS] {
            for index in 0..HEADINGS / 2 {
                for (frame, opposite) in frames(spec, clips[index]).into_iter().zip(frames(spec, clips[index + HEADINGS / 2])) {
                    let (low, high) = frame_box(&image, spec, frame).expect("a heading draws");
                    let (opposite_low, opposite_high) = frame_box(&image, spec, opposite).expect("a heading draws");
                    let (turned_low, turned_high) = (centre * 2.0 - high, centre * 2.0 - low);
                    assert!(
                        (turned_low - opposite_low).abs().max_element() <= 1.0
                            && (turned_high - opposite_high).abs().max_element() <= 1.0,
                        "{} {}: turned box {turned_low}..{turned_high} against {opposite_low}..{opposite_high}",
                        spec.path, clips[index],
                    );
                }
            }
        }
    }
}

/// A shot leaves just past the spout: the spout's tip reaches `FUNK_SPOUT_REACH` along
/// every heading, and nothing of the ship reaches past `FUNK_REACH`, its wrap margin.
#[test]
fn the_spout_and_the_plume_reach_what_the_game_measured() {
    for spec in FUNKS {
        let image = pixels(spec);
        let centre = spec.cell * 0.5;
        for (index, clip) in IDLE_HEADINGS.iter().enumerate() {
            let angle = index as f32 * std::f32::consts::TAU / HEADINGS as f32;
            let direction = Vec2::new(angle.cos(), -angle.sin()); // cell Y grows down
            let tip = frames(spec, clip).into_iter()
                .flat_map(|frame| opaque_pixels(&image, spec, frame))
                .map(|pixel| (pixel - centre).dot(direction))
                .fold(f32::MIN, f32::max);
            assert!((FUNK_SPOUT_REACH - 1.0..=FUNK_SPOUT_REACH).contains(&tip), "{} {clip}: the spout reaches {tip}", spec.path);
        }
        let flying: Vec<u32> = IDLE_HEADINGS.iter().chain(THRUST_HEADINGS.iter())
            .flat_map(|clip| frames(spec, clip)).collect();
        let reach = farthest_reach(&image, spec, &flying, centre);
        assert!(reach <= FUNK_REACH && reach > FUNK_REACH - 1.0, "{}: the ship reaches {reach}", spec.path);
        const { assert!(MUZZLE_OFFSET > FUNK_SPOUT_REACH, "a shot leaves clear of the spout") };
    }
}

#[test]
fn player_twos_funk_has_player_ones_mask_in_every_frame() {
    let (one, two) = (pixels(&FLYING_FUNK), pixels(&FLYING_FUNK_PLAYER_TWO));
    assert_eq!(one.dimensions(), two.dimensions());
    let frame_count = (one.width() / 64) * (one.height() / 64);
    for frame in 0..frame_count {
        assert_eq!(
            opaque_pixels(&one, &FLYING_FUNK, frame), opaque_pixels(&two, &FLYING_FUNK_PLAYER_TWO, frame),
            "frame {frame}'s silhouettes differ",
        );
    }
    assert_ne!(one.as_raw(), two.as_raw(), "player 2's Funk is recoloured");
}

/// The crash is drawn nose east about the same pivot, and ends empty.
#[test]
fn the_crash_ends_empty() {
    for spec in FUNKS {
        let image = pixels(spec);
        let crash = frames(spec, CRASH);
        assert_eq!(crash.len(), 6);
        assert!(frame_box(&image, spec, *crash.last().expect("frames")).is_none(), "{}'s crash ends empty", spec.path);
    }
}

/// Each meatieroid's body is a circle about the stated centre in every `idle` and
/// `attack` frame — the column through the centre spans it — at the collider's radius in
/// its rest frame and within a pixel of it while it breathes and flexes. The sheet's
/// bounds are that circle's box, and nothing a live meatieroid draws, fists included,
/// reaches past the wrap margin.
#[test]
fn each_meatieroid_body_is_its_collider_circle() {
    for size in SIZES {
        let spec = size.art().sheet;
        let image = pixels(spec);
        let (top_left, bottom_right) = spec.bounds;
        let centre = (top_left + bottom_right) * 0.5;
        assert_eq!((bottom_right - top_left) * 0.5, Vec2::splat(size.radius()), "{size:?}'s bounds are its body circle's box");

        let rest = frames(spec, IDLE)[0];
        for frame in frames(spec, IDLE).into_iter().chain(frames(spec, ATTACK)) {
            let column: Vec<f32> = opaque_pixels(&image, spec, frame).into_iter()
                .filter(|pixel| (pixel.x - centre.x).abs() < 1.0)
                .map(|pixel| pixel.y)
                .collect();
            let top = column.iter().copied().fold(f32::MAX, f32::min) - 0.5;
            let bottom = column.iter().copied().fold(f32::MIN, f32::max) + 0.5;
            assert_eq!((top + bottom) * 0.5, centre.y, "{size:?} frame {frame}: the body's centre moved");
            let radius = (bottom - top) * 0.5;
            if frame == rest {
                assert_eq!(radius, size.radius(), "{size:?}'s rest body");
            } else {
                assert!((radius - size.radius()).abs() <= 1.0, "{size:?} frame {frame}: body radius {radius}");
            }
        }

        // A `die` plays on a detached one-shot that never wraps, so only the live frames count.
        let live_frames: Vec<u32> = [IDLE, ATTACK].iter().flat_map(|clip| frames(spec, clip)).collect();
        let reach = farthest_reach(&image, spec, &live_frames, centre);
        let margin = MEATIEROID_REACH[size.idx()];
        assert!(reach <= margin && reach > margin - 1.0, "{size:?}: a fist reaches {reach}, the margin is {margin}");
    }
}

#[test]
fn the_shots_boxes_are_their_fly_frames() {
    for spec in [&ICICLE, &ICE_CHIP] {
        let image = pixels(spec);
        let measured = frames(spec, FLY).into_iter()
            .filter_map(|frame| frame_box(&image, spec, frame))
            .reduce(|(min, max), (low, high)| (min.min(low), max.max(high)))
            .expect("a shot draws");
        assert_eq!(measured, spec.bounds, "{}", spec.path);
        let half_long_side = (spec.bounds.1 - spec.bounds.0).max_element() * 0.5;
        assert!(SHOT_REACH >= half_long_side, "{}: a shot wraps before it is off the field", spec.path);
    }
    // The icicle is drawn tip up: its top row is narrower than its bottom.
    let image = pixels(&ICICLE);
    let pixels_on = |row: f32| opaque_pixels(&image, &ICICLE, 0).into_iter().filter(|pixel| pixel.y == row).count();
    assert!(pixels_on(ICICLE.bounds.0.y + 0.5) < pixels_on(ICICLE.bounds.1.y - 0.5), "the icicle points up");
}

/// The stars sit on a transparent ground, so the grid and the night show between them.
#[test]
fn the_space_tile_is_mostly_transparent() {
    let image = pixels(&SPACE);
    assert_eq!(image.dimensions(), (SPACE_TILE_PX as u32, SPACE_TILE_PX as u32));
    let opaque = opaque_pixels(&image, &SPACE, 0).len();
    assert!(opaque > 0 && opaque < (SPACE_TILE_PX * SPACE_TILE_PX) as usize / 10, "{opaque} opaque pixels");
}

#[test]
fn the_night_is_the_style_guides_hex_decoded() {
    let [red, green, blue, alpha] = night_clear_color();
    assert_eq!(alpha, 1.0);
    // #241418 decoded from sRGB: dark, with red over blue over green as the hex reads.
    assert!((red - 0.0176).abs() < 0.001 && (green - 0.0070).abs() < 0.001 && (blue - 0.0091).abs() < 0.001,
        "the night decodes to {red} {green} {blue}");
}
