//! Visual effect presets — particle configs.
//!
//! Centralizes the look of each event (a meatieroid busted, a Funk crashed) so
//! tuning happens in one place. The drawn one-shots (`die`, `crash`) play beside
//! these; the deforming grid is the engine's `GridBackdrop`.

use engine_core::prelude::*;

use crate::art::MEAT_BURST_COLOR;
use crate::types::AsteroidSize;

/// Crumbs of meat when a meatieroid bursts — bigger ones throw more.
pub(crate) fn asteroid_break_burst(size: AsteroidSize, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let base = match size {
        AsteroidSize::Large => 32.0,
        AsteroidSize::Medium => 20.0,
        AsteroidSize::Small => 12.0,
    };
    let count = (base * theme.particle_count_mult).round() as usize;
    let color = MEAT_BURST_COLOR;
    ParticleConfig::burst(count)
        .with_lifetime(0.25, 0.7)
        .with_speed(60.0, 260.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(5.0, 0.5)
        .with_drag(2.0)
        .with_emissive(2.0)
        .with_texture(tex)
}

/// A big blowout in the Funk's own colour when it goes down.
pub(crate) fn ship_death_burst(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (70.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.35, 1.0)
        .with_speed(120.0, 480.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(8.0, 0.5)
        .with_drag(1.8)
        .with_emissive(2.8)
        .with_texture(tex)
}
