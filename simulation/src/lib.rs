use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

mod arena;
pub mod driver;
pub mod prelude;
mod road;
mod spawner;

/// Log to console (works in both native and WASM)
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!($($arg)*).into());
        #[cfg(not(target_arch = "wasm32"))]
        println!($($arg)*);
    }};
}

pub use arena::*;
use bevy_time::Time;
pub use road::*;
pub use spawner::*;

use crate::driver::{
    apply_gap_acceptance, apply_idm, move_and_despawn_vehicles, spawn_vehicles, update_occupancy,
    SegmentOccupancy, Vehicle,
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
                apply_gap_acceptance,
                apply_idm,
                move_and_despawn_vehicles,
            )
                .chain(),
        );
    }
}
