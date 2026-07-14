//! Vector-graphics rendering: the ship and rocks are neon wireframes pushed
//! into `ctx.lines` (the same line-list pipeline the deforming grid uses),
//! not sprites. Pure geometry helpers live up top so tests can pin them
//! without a world.
//!
//! Convention: local space puts the nose along +x, matching
//! `Transform2D::forward()` = (cos r, sin r).

use engine_core::prelude::*;
use std::f32::consts::TAU;

use crate::constants::*;
use crate::gameplay::blink_visible;
use crate::types::*;

/// Rotate `local` by `rot` and translate to `pos`.
pub(crate) fn transform_point(local: Vec2, pos: Vec2, rot: f32) -> Vec2 {
    let (s, c) = rot.sin_cos();
    Vec2::new(local.x * c - local.y * s, local.x * s + local.y * c) + pos
}

/// Ship hull in local space: nose, left wing, rear notch, right wing.
pub(crate) fn ship_points() -> [Vec2; 4] {
    [
        Vec2::new(SHIP_SIZE, 0.0),
        Vec2::new(-0.7 * SHIP_SIZE, 0.65 * SHIP_SIZE),
        Vec2::new(-0.4 * SHIP_SIZE, 0.0),
        Vec2::new(-0.7 * SHIP_SIZE, -0.65 * SHIP_SIZE),
    ]
}

/// The four hull segments as a closed loop in world space.
pub(crate) fn ship_segments(pos: Vec2, rot: f32) -> [(Vec2, Vec2); 4] {
    let p = ship_points().map(|l| transform_point(l, pos, rot));
    [(p[0], p[1]), (p[1], p[2]), (p[2], p[3]), (p[3], p[0])]
}

/// Exhaust flame behind the rear notch; `flicker` in [0, 1) jitters the tip
/// length every frame.
pub(crate) fn thrust_flame_segments(pos: Vec2, rot: f32, flicker: f32) -> [(Vec2, Vec2); 2] {
    let tip = transform_point(Vec2::new(-SHIP_SIZE * (1.1 + 0.5 * flicker), 0.0), pos, rot);
    let left = transform_point(Vec2::new(-0.55 * SHIP_SIZE, 0.3 * SHIP_SIZE), pos, rot);
    let right = transform_point(Vec2::new(-0.55 * SHIP_SIZE, -0.3 * SHIP_SIZE), pos, rot);
    [(left, tip), (right, tip)]
}

/// Irregular rock outline: `ASTEROID_VERTS` points at evenly spaced angles
/// with hash-jittered radii. Deterministic per seed — generated once at
/// spawn and stored on the `Asteroid`.
pub(crate) fn asteroid_polygon(seed: u32, radius: f32) -> Vec<Vec2> {
    (0..ASTEROID_VERTS)
        .map(|k| {
            let angle = k as f32 / ASTEROID_VERTS as f32 * TAU;
            let jitter = hash_f32(seed.wrapping_mul(31).wrapping_add(k as u32));
            let r = radius * (0.72 + 0.46 * jitter);
            Vec2::new(angle.cos(), angle.sin()) * r
        })
        .collect()
}

/// Append world-space segments as line-list vertex pairs.
pub(crate) fn push_segments(
    lines: &mut Vec<LineVertex>,
    segs: &[(Vec2, Vec2)],
    color: Vec4,
    emissive: f32,
) {
    for &(a, b) in segs {
        lines.push(LineVertex::new(a, color, emissive));
        lines.push(LineVertex::new(b, color, emissive));
    }
}

/// Append a closed local-space polygon, transformed to world space.
pub(crate) fn push_closed_polygon(
    lines: &mut Vec<LineVertex>,
    local: &[Vec2],
    pos: Vec2,
    rot: f32,
    color: Vec4,
    emissive: f32,
) {
    let n = local.len();
    for i in 0..n {
        let a = transform_point(local[i], pos, rot);
        let b = transform_point(local[(i + 1) % n], pos, rot);
        lines.push(LineVertex::new(a, color, emissive));
        lines.push(LineVertex::new(b, color, emissive));
    }
}

impl AsteroidsGame {
    /// Draw the frame's wireframes. Called after `step_and_emit_grid` so
    /// gameplay lines land on top of the grid. Rocks keep drifting behind
    /// the game-over overlay; the ship only exists while Playing (and skips
    /// frames to blink during i-frames).
    pub(crate) fn emit_wireframes(&self, ctx: &mut GameContext) {
        if !matches!(self.state, GameState::Playing | GameState::GameOver) {
            return;
        }
        let theme = ChaosTheme::for_mode(self.chaos_mode);

        for rock in &self.asteroids {
            if let Some(t) = ctx.world.get::<Transform2D>(rock.entity) {
                push_closed_polygon(
                    ctx.lines, &rock.shape, t.position, t.rotation,
                    theme.structure_color, ASTEROID_EMISSIVE,
                );
            }
        }

        if self.state != GameState::Playing || !blink_visible(self.invincibility) {
            return;
        }
        let Some(ship) = self.ship else { return };
        let Some((pos, rot)) = ctx.world.get::<Transform2D>(ship).map(|t| (t.position, t.rotation))
        else {
            return;
        };
        push_segments(ctx.lines, &ship_segments(pos, rot), theme.accent_color, SHIP_EMISSIVE);
        if self.thrusting {
            push_segments(
                ctx.lines,
                &thrust_flame_segments(pos, rot, hash_f32(self.frame_count)),
                FLAME_COLOR,
                FLAME_EMISSIVE,
            );
        }
    }
}
