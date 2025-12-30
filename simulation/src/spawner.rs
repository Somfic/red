use bevy_ecs::prelude::*;
use bevy_time::Time;

use crate::{driver::Idm, Id, Segment, Vehicle};

/// Spawns vehicles at a regular interval on a specific segment
#[derive(Component)]
pub struct VehicleSpawner {
    /// The segment where vehicles will spawn
    pub segment: Id<Segment>,
    /// Vehicles spawned per second
    pub rate: f32,
    /// Time until next spawn
    pub timer: f32,
    /// Speed of spawned vehicles
    pub vehicle_speed: f32,
}

impl VehicleSpawner {
    pub fn new(segment: Id<Segment>, rate: f32) -> Self {
        Self {
            segment,
            rate,
            timer: 1.0 / rate,
            vehicle_speed: 2.0,
        }
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.vehicle_speed = speed;
        self
    }
}
