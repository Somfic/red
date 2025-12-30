use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

mod arena;
pub mod driver;
pub mod prelude;
mod road;
mod spawner;

pub use arena::*;
use bevy_time::Time;
pub use road::*;
pub use spawner::*;

use crate::driver::{apply_idm, update_occupancy, SegmentOccupancy, Vehicle};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SegmentOccupancy>();

        app.add_systems(
            Update,
            (spawn_vehicles, update_occupancy, apply_idm, move_vehicles).chain(),
        );
    }
}

fn move_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    mut vehicles: Query<(Entity, &mut Vehicle)>,
    roads: Res<Road>,
) {
    for (entity, mut vehicle) in &mut vehicles {
        let segment = roads.segments.get(&vehicle.segment);
        let from = roads.nodes.get(&segment.from);
        let to = roads.nodes.get(&segment.to);

        let segment_length = from.position.distance(to.position);
        let progress_delta = vehicle.speed * time.delta_secs() / segment_length;

        vehicle.progress += progress_delta;

        // move to the next segment
        if vehicle.progress >= 1.0 {
            if to.outgoing.is_empty() {
                commands.entity(entity).despawn();
            } else {
                vehicle.segment = *to.outgoing.first().unwrap();
                vehicle.progress -= 1.0;
            }
        }
    }
}
