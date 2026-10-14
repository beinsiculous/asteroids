//! All entity creation. Every entity gets a `Name` component so the editor
//! hierarchy shows "Meatieroid Large #7" instead of "Entity 7".
//!
//! Every art entity is built the same way: the sheet's first cell, the cell-sized
//! transform scale, and the sheet's measured anchor as the sprite offset, so the art
//! lands on the entity rather than at the cell's corner. Colliders are the physics
//! truth (F1 shows the overlay).

use engine_core::prelude::*;

use crate::art::*;
use crate::constants::*;
use crate::gameplay::{asteroid_speed, live_ship_positions, spawn_position, wave_asteroid_count};
use crate::types::*;

/// The components every art entity carries: the sheet's first cell as its sprite at
/// `depth`, the sheet's clips as its animation, and the anchor offset that puts the
/// measured box on the entity. The engine turns the offset with the entity's rotation,
/// so a tumbling meatieroid turns about its body.
pub(crate) fn art_components(sheet: &SpriteSheet, spec: &SheetSpec, depth: f32) -> (Sprite, SpriteAnimation) {
    (sheet.sprite().with_offset(spec.sprite_offset()).with_depth(depth), sheet.animation())
}

/// Where a ship starts, given the player count and its 0-based index, and the heading
/// it starts on. Single player sits at the center; co-op puts the two ships side by
/// side on the x axis, both facing up, so neither starts on top of a meatieroid.
pub(crate) fn ship_spawn_pose(player_count: usize, index: usize) -> (Vec2, f32) {
    let facing = std::f32::consts::FRAC_PI_2;
    if player_count <= 1 {
        (Vec2::ZERO, facing)
    } else {
        let x = if index == 0 { -COOP_SHIP_SPAWN_X } else { COOP_SHIP_SPAWN_X };
        (Vec2::new(x, 0.0), facing)
    }
}

/// Spawn `pilot`'s Funk at `position`. Dynamic with its rotation locked: the heading
/// lives in `ShipState` and the sprite shows it through the drawn headings, so the body
/// never turns. The collider is a sensor: meatieroids must never physically deflect
/// the ship, and the two co-op ships pass cleanly through each other (deaths are
/// resolved by game code, which respects i-frames).
pub(crate) fn spawn_ship(world: &mut World, sheets: &Sheets, pilot: Pilot, position: Vec2) -> EntityId {
    let art = pilot.art();
    let (sprite, animation) = art_components(sheets.funk(pilot), art.sheet, art.depth);
    world.spawn()
        .with(Name::new(art.name))
        .with(Transform2D::from_parts(position, 0.0, art.sheet.scale()))
        .with(sprite)
        .with(animation)
        .with(RigidBody::new_dynamic()
            .with_gravity_scale(0.0)
            .with_rotation_locked(true)
            .with_linear_damping(0.0)
            .with_angular_damping(0.0))
        .with(Collider::circle_collider(SHIP_COLLIDER_RADIUS).as_sensor())
        .id()
}

/// A wave's meatieroid loops `idle`.
pub(crate) fn wave_machine() -> ClipStateMachine {
    ClipStateMachine::single(IDLE)
}

/// A split's child is born flexing: `attack` plays once, then it loops `idle`.
pub(crate) fn child_machine() -> ClipStateMachine {
    ClipStateMachine::new(ATTACK, vec![
        (ATTACK.to_string(), ClipState::new(ATTACK, OnFinished::Next(IDLE.to_string()))),
        (IDLE.to_string(), ClipState::staying(IDLE)),
    ])
}

/// What a detached one-shot draws and where: a meatieroid's `die`, a Funk's `crash`.
pub(crate) struct Effect<'a> {
    pub(crate) name: &'a str,
    pub(crate) spec: &'a SheetSpec,
    pub(crate) sheet: &'a SpriteSheet,
    pub(crate) depth: f32,
    pub(crate) clip: &'a str,
    pub(crate) position: Vec2,
    pub(crate) rotation: f32,
}

/// Spawn a detached one-shot: sprite-only, no collider, ending itself when its clip
/// does. The caller keeps the handle in `transient_visuals`.
pub(crate) fn spawn_effect(world: &mut World, effect: &Effect) -> EntityId {
    let (sprite, animation) = art_components(effect.sheet, effect.spec, effect.depth);
    world.spawn()
        .with(Name::new(effect.name))
        .with(Transform2D::from_parts(effect.position, effect.rotation, effect.spec.scale()))
        .with(sprite)
        .with(animation)
        .with(ClipStateMachine::one_shot(effect.clip))
        .id()
}

/// Spawn the stars: a `Tilemap` of the space tile, `ceil(WIN / tile)` cells each way,
/// centred on the window. Tiles are placed in pixel units from the entity's position,
/// which anchors the centre of tile (0, 0) — the top-left cell, with rows growing down.
pub(crate) fn spawn_space(world: &mut World, sheet: &SpriteSheet) -> EntityId {
    let columns = (WIN_W / SPACE_TILE_PX).ceil() as u32;
    let rows = (WIN_H / SPACE_TILE_PX).ceil() as u32;

    let mut map = Tilemap::new(columns, rows, SPACE_TILE_PX);
    map.tileset = sheet.texture.id;
    map.tile_uv_size = Vec2::new(1.0 / sheet.grid.cols as f32, 1.0 / sheet.grid.rows as f32);
    map.tiles = vec![1; (columns * rows) as usize];
    map.depth = SPACE_DEPTH;

    let anchor = Vec2::new(
        -(columns as f32 - 1.0) * SPACE_TILE_PX / 2.0,
        (rows as f32 - 1.0) * SPACE_TILE_PX / 2.0,
    );
    world.spawn().with(Name::new("Space")).with(Transform2D::new(anchor)).with(map).id()
}

/// Spawn the deforming grid: the engine simulates it and draws it behind the sprites,
/// over the night and under the stars; gameplay events ripple it.
pub(crate) fn spawn_backdrop(world: &mut World, theme: &ChaosTheme) -> EntityId {
    world.spawn()
        .with(Name::new("Grid Backdrop"))
        .with(Transform2D::new(Vec2::ZERO))
        .with(GridBackdrop {
            draw_order: GridDrawOrder::BehindSprites,
            color: theme.grid_color,
            ..GridBackdrop::default()
        })
        .id()
}

/// The shot's turn for a flight along `velocity`: the art is drawn pointing up.
pub(crate) fn shot_rotation(velocity: Vec2) -> f32 {
    velocity.y.atan2(velocity.x) - std::f32::consts::FRAC_PI_2
}

impl AsteroidsGame {
    /// Next value of the monotonic hash-seed stream. Every random-looking
    /// decision (speed, spin, spawn point, split jitter) draws from this.
    pub(crate) fn next_seed(&mut self) -> u32 {
        self.spawn_counter = self.spawn_counter.wrapping_add(1);
        self.spawn_counter
    }

    /// Spawn one meatieroid: a solid dynamic body that bounces elastically off the others
    /// and tumbles with its spin, drawn on its size's sheet and running `machine`.
    pub(crate) fn spawn_asteroid(
        &mut self,
        world: &mut World,
        size: AsteroidSize,
        pos: Vec2,
        vel: Vec2,
        spin: f32,
        machine: ClipStateMachine,
    ) -> EntityId {
        let seed = self.next_seed();
        let art = size.art();
        let (sprite, animation) = art_components(self.sheets.meatieroid(size), art.sheet, art.depth);
        let entity = world.spawn()
            .with(Name::new(format!("Meatieroid {size:?} #{seed}")))
            .with(Transform2D::from_parts(pos, 0.0, art.sheet.scale()))
            .with(sprite)
            .with(animation)
            .with(machine)
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0))
            .with(Collider::circle_collider(size.radius())
                .with_restitution(1.0)
                .with_friction(0.0))
            .id();
        // Buffered-safe on the spawn frame; applied once the body syncs.
        self.physics.set_velocity(entity, vel, spin);
        self.asteroids.push(Asteroid { entity, size });
        entity
    }

    /// Spawn one shot owned by ship `ship_index`: a dynamic sensor flying straight with a
    /// `Lifetime` safety net (the engine despawns strays, which also caps the range). Its
    /// turn is written here, once, before the body first syncs: the body's rotation is
    /// locked, and physics reads a later change to the transform's rotation as an outside
    /// edit and teleports the body.
    pub(crate) fn spawn_bullet(&mut self, world: &mut World, ship_index: usize, pos: Vec2, velocity: Vec2) {
        let pilot = Pilot::for_ship(ship_index);
        let art = pilot.art();
        let (sprite, animation) = art_components(self.sheets.shot(pilot), art.shot, art.shot_depth);
        let entity = world.spawn()
            .with(Name::new("Shot"))
            .with(Transform2D::from_parts(pos, shot_rotation(velocity), art.shot.scale()))
            .with(sprite)
            .with(animation)
            .with(ClipStateMachine::single(FLY))
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_rotation_locked(true)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0))
            .with(Collider::circle_collider(BULLET_RADIUS).as_sensor())
            .with(Lifetime::new(BULLET_LIFETIME))
            .id();
        self.physics.set_velocity(entity, velocity, 0.0);
        self.ships[ship_index].bullets.push(entity);
    }

    /// Spawn the current wave: large meatieroids on the window border, never closer than
    /// `SAFE_SPAWN_DIST` to ANY live ship, drifting in hash-seeded directions.
    pub(crate) fn spawn_wave(&mut self, world: &mut World) {
        let avoid = live_ship_positions(&self.ships, world);
        let count = wave_asteroid_count(self.wave, self.chaos_mode);
        for _ in 0..count {
            let pos = spawn_position(self.next_seed(), &avoid);
            let speed = asteroid_speed(AsteroidSize::Large, self.next_seed(), self.chaos_mode);
            let angle = hash_f32(self.next_seed()) * std::f32::consts::TAU;
            let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
            let spin = (hash_f32(self.next_seed()) - 0.5) * 2.0 * ASTEROID_SPIN_MAX;
            self.spawn_asteroid(world, AsteroidSize::Large, pos, vel, spin, wave_machine());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_carries_a_sensor_circle_collider_and_a_locked_rotation() {
        let mut world = World::new();
        let ship = spawn_ship(&mut world, &Sheets::default(), Pilot::Deion, Vec2::ZERO);
        let collider = world.get::<Collider>(ship).expect("ship needs a collider");
        assert!(collider.is_sensor, "meatieroids must never physically deflect the ship");
        let body = world.get::<RigidBody>(ship).expect("ship needs a body");
        assert!(!body.can_rotate, "the drawn headings turn the ship, never the body");
        assert_eq!(world.get::<Transform2D>(ship).map(|t| t.rotation), Some(0.0));
    }

    #[test]
    fn spawned_asteroid_registers_its_size_and_tracking() {
        let mut game = AsteroidsGame::default();
        let mut world = World::new();
        let entity = game.spawn_asteroid(
            &mut world, AsteroidSize::Large, Vec2::ZERO, Vec2::X, 0.0, wave_machine());
        assert_eq!(game.asteroids.len(), 1);
        assert_eq!(game.asteroids[0].entity, entity);
        assert_eq!(game.asteroids[0].size, AsteroidSize::Large);
    }

    #[test]
    fn bullets_carry_lifetime_and_sensor() {
        let mut game = AsteroidsGame::default();
        game.ships.push(ShipState::default());
        let mut world = World::new();
        game.spawn_bullet(&mut world, 0, Vec2::ZERO, Vec2::Y * BULLET_SPEED);
        let bullet = game.ships[0].bullets[0];
        assert!(world.get::<Lifetime>(bullet).is_some(), "bullets must carry a Lifetime");
        let collider = world.get::<Collider>(bullet).expect("bullet needs a collider");
        assert!(collider.is_sensor, "bullets must not push meatieroids around");
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
