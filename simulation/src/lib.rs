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

use crate::driver::{
    apply_idm, move_and_despawn_vehicles, spawn_vehicles, update_occupancy, SegmentOccupancy,
    Vehicle,
};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SegmentOccupancy>();

        app.add_systems(
            Update,
            (
                spawn_vehicles,
                update_occupancy,
                apply_idm,
                move_and_despawn_vehicles,
            )
                .chain(),
        );
    }
}
