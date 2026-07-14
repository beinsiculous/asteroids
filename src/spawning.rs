//! All entity creation. Every entity gets a `Name` component so the editor
//! hierarchy shows "Asteroid Large #7" instead of "Entity 7".
//!
//! The ship and asteroids carry NO sprites — they render as wireframes (see
//! `wireframe.rs`); their circle colliders are the physics truth (F1 shows
//! the overlay). Bullets stay tiny emissive sprites.

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::{asteroid_speed, spawn_position, wave_asteroid_count};
use crate::types::*;
use crate::wireframe;

/// Spawn the player ship at the center, facing up. Dynamic with free
/// rotation — gameplay drives it with one `set_velocity` per frame. The
/// collider is a sensor: rocks must never physically deflect the ship
/// (deaths are resolved by game code, which respects i-frames).
pub(crate) fn spawn_ship(world: &mut World) -> EntityId {
    world.spawn()
        .with(Name::new("Ship"))
        .with(Transform2D::from_parts(Vec2::ZERO, std::f32::consts::FRAC_PI_2, Vec2::ONE))
        .with(RigidBody::new_dynamic()
            .with_gravity_scale(0.0)
            .with_linear_damping(0.0)
            .with_angular_damping(0.0))
        .with(Collider::circle_collider(SHIP_COLLIDER_RADIUS).as_sensor())
        .id()
}

impl AsteroidsGame {
    /// Next value of the monotonic hash-seed stream. Every random-looking
    /// decision (shape, speed, spin, spawn point) draws from this.
    pub(crate) fn next_seed(&mut self) -> u32 {
        self.spawn_counter = self.spawn_counter.wrapping_add(1);
        self.spawn_counter
    }

    /// Spawn one rock: a solid dynamic body that bounces elastically off
    /// other rocks, plus its hash-seeded wireframe outline.
    pub(crate) fn spawn_asteroid(
        &mut self,
        world: &mut World,
        size: AsteroidSize,
        pos: Vec2,
        vel: Vec2,
        spin: f32,
    ) -> EntityId {
        let seed = self.next_seed();
        let shape = wireframe::asteroid_polygon(seed, size.radius());
        let entity = world.spawn()
            .with(Name::new(format!("Asteroid {size:?} #{seed}")))
            .with(Transform2D::from_parts(pos, 0.0, Vec2::ONE))
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0))
            .with(Collider::circle_collider(size.radius() * ASTEROID_COLLIDER_SCALE)
                .with_restitution(1.0)
                .with_friction(0.0))
            .id();
        // Buffered-safe on the spawn frame; applied once the body syncs.
        self.physics.set_velocity(entity, vel, spin);
        self.asteroids.push(Asteroid { entity, size, shape });
        entity
    }

    /// Spawn one bullet: a dynamic sensor flying along the ship's heading
    /// with a `Lifetime` safety net (the engine despawns strays, which also
    /// caps bullet range).
    pub(crate) fn spawn_bullet(&mut self, world: &mut World, pos: Vec2, velocity: Vec2) {
        let entity = world.spawn()
            .with(Name::new("Bullet"))
            .with(Transform2D::from_parts(pos, 0.0, Vec2::splat(BULLET_SIZE_PX / RENDER_UNIT)))
            .with(Sprite::new(self.tex_id).with_color(BULLET_COLOR).with_emissive(BULLET_EMISSIVE))
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_rotation_locked(true)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0))
            .with(Collider::circle_collider(BULLET_RADIUS).as_sensor())
            .with(Lifetime::new(BULLET_LIFETIME))
            .id();
        self.physics.set_velocity(entity, velocity, 0.0);
        self.bullets.push(entity);
    }

    /// Spawn the current wave: large rocks on the window border, never
    /// closer than `SAFE_SPAWN_DIST` to the ship, drifting in hash-seeded
    /// directions.
    pub(crate) fn spawn_wave(&mut self, world: &mut World, ship_pos: Vec2) {
        let count = wave_asteroid_count(self.wave, self.chaos_mode);
        for _ in 0..count {
            let pos = spawn_position(self.next_seed(), ship_pos);
            let speed = asteroid_speed(AsteroidSize::Large, self.next_seed(), self.chaos_mode);
            let angle = hash_f32(self.next_seed()) * std::f32::consts::TAU;
            let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
            let spin = (hash_f32(self.next_seed()) - 0.5) * 2.0 * ASTEROID_SPIN_MAX;
            self.spawn_asteroid(world, AsteroidSize::Large, pos, vel, spin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_carries_a_sensor_circle_collider() {
        let mut world = World::new();
        let ship = spawn_ship(&mut world);
        let collider = world.get::<Collider>(ship).expect("ship needs a collider");
        assert!(collider.is_sensor, "rocks must never physically deflect the ship");
    }

    #[test]
    fn spawned_asteroid_registers_shape_and_tracking() {
        let mut game = AsteroidsGame::default();
        let mut world = World::new();
        let entity = game.spawn_asteroid(
            &mut world, AsteroidSize::Large, Vec2::ZERO, Vec2::X, 0.0);
        assert_eq!(game.asteroids.len(), 1);
        assert_eq!(game.asteroids[0].entity, entity);
        assert_eq!(game.asteroids[0].shape.len(), ASTEROID_VERTS);
    }

    #[test]
    fn bullets_carry_lifetime_and_sensor() {
        let mut game = AsteroidsGame::default();
        let mut world = World::new();
        game.spawn_bullet(&mut world, Vec2::ZERO, Vec2::Y * BULLET_SPEED);
        let bullet = game.bullets[0];
        assert!(world.get::<Lifetime>(bullet).is_some(), "bullets must carry a Lifetime");
        let collider = world.get::<Collider>(bullet).expect("bullet needs a collider");
        assert!(collider.is_sensor, "bullets must not push rocks around");
    }

    #[test]
    fn seed_stream_is_monotonic() {
        let mut game = AsteroidsGame::default();
        let a = game.next_seed();
        let b = game.next_seed();
        assert_ne!(a, b);
        assert_eq!(b, a + 1);
    }
}
