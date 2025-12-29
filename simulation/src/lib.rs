use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_time::Time;
use glam::Vec3;

pub mod prelude;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_vehicles);
    }
}

#[derive(Component)]
pub struct Node {
    pub position: Vec3,
}

#[derive(Component)]
pub struct Segment {
    pub from: Entity,
    pub to: Entity,
}

#[derive(Component)]
pub struct Vehicle {
    pub speed: f32,
}

#[derive(Component)]
pub struct OnSegment {
    pub segment: Entity,
    pub progress: f32, // 0.0 = at start, 1.0 = at end
}

#[derive(Component)]
pub struct TrafficLight {
    pub state: LightState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightState {
    Red,
    Yellow,
    Green,
}

fn move_vehicles(
    time: Res<Time>,
    mut vehicles: Query<(&Vehicle, &mut OnSegment)>,
    segments: Query<&Segment>,
    nodes: Query<&Node>,
) {
    for (vehicle, mut on_segment) in &mut vehicles {
        let Ok(segment) = segments.get(on_segment.segment) else {
            continue;
        };

        let Ok(from_node) = nodes.get(segment.from) else {
            continue;
        };

        let Ok(to_node) = nodes.get(segment.to) else {
            continue;
        };

        let segment_length = from_node.position.distance(to_node.position);
        let progress_delta = vehicle.speed * time.delta_secs() / segment_length;

        on_segment.progress += progress_delta;

        if on_segment.progress >= 1.0 {
            on_segment.progress = 0.0;
        }
    }
}
