//! Visual effect presets — particle configs.
//!
//! Centralizes the look of each event (rock shattered, ship destroyed) so
//! tuning happens in one place. The deforming grid uses the engine's
//! `default_playfield_grid` preset directly.

use engine_core::prelude::*;

use crate::types::AsteroidSize;

/// Stony shrapnel when a rock shatters — bigger rocks throw more debris.
pub(crate) fn asteroid_break_burst(size: AsteroidSize, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let base = match size {
        AsteroidSize::Large => 32.0,
        AsteroidSize::Medium => 20.0,
        AsteroidSize::Small => 12.0,
    };
    let count = (base * theme.particle_count_mult).round() as usize;
    let color = theme.structure_color;
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

/// Big accent-colored blowout when the ship goes down.
pub(crate) fn ship_death_burst(theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let color = theme.accent_color;
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
