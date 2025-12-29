use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

mod arena;
pub mod prelude;
mod road;
pub use arena::*;
use bevy_time::Time;
pub use road::*;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_vehicles);
    }
}

#[derive(Component)]
pub struct Vehicle {
    pub speed: f32,
}

#[derive(Component)]
pub struct OnSegment {
    pub segment: Id<Segment>,
    pub progress: f32,
}

fn move_vehicles(
    time: Res<Time>,
    mut vehicles: Query<(&Vehicle, &mut OnSegment)>,
    roads: Res<Road>,
) {
    for (vehicle, mut on_segment) in &mut vehicles {
        let segment = roads.segments.get(&on_segment.segment);
        let from = roads.nodes.get(&segment.from);
        let to = roads.nodes.get(&segment.to);

        let segment_length = from.position.distance(to.position);
        let progress_delta = vehicle.speed * time.delta_secs() / segment_length;

        on_segment.progress += progress_delta;

        // move to the next segment
        if on_segment.progress >= 1.0 {
            if to.outgoing.is_empty() {
                on_segment.progress = 1.0;
            } else {
                on_segment.segment = *to.outgoing.first().unwrap();
                on_segment.progress -= 1.0;
            }
        }
    }
}
