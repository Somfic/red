use crate::{
    driver::{Idm, SegmentOccupancy},
    Id, Road, Segment,
};
use bevy_ecs::prelude::*;
use bevy_time::Time;

#[derive(Component)]
pub struct Vehicle {
    pub speed: f32,
    pub segment: Id<Segment>,
    pub progress: f32,
    pub idm: Idm,
}

impl Vehicle {
    pub fn new(segment: Id<Segment>) -> Self {
        Self {
            speed: 0.0,
            segment,
            progress: 0.0,
            idm: Idm::new(rand::random()),
        }
    }
}

/// Marker component for the player-controlled vehicle
#[derive(Component)]
pub struct PlayerControlled;

pub fn move_and_despawn_vehicles(
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
                // Filter out U-turns (segments going back to where we came from)
                let came_from = segment.from;
                let valid_segments: Vec<_> = to
                    .outgoing
                    .iter()
                    .filter(|&&seg_id| roads.segments.get(&seg_id).to != came_from)
                    .collect();

                if valid_segments.is_empty() {
                    // No valid turns, despawn (or allow U-turn as fallback)
                    commands.entity(entity).despawn();
                } else {
                    let turn_index =
                        (rand::random::<f32>() * valid_segments.len() as f32).floor() as usize;
                    vehicle.segment = *valid_segments[turn_index];
                    vehicle.progress -= 1.0;
                }
            }
        }
    }
}

pub fn spawn_vehicles(mut commands: Commands, roads: Res<Road>, occupancy: Res<SegmentOccupancy>) {
    let mut total_vehicles: usize = occupancy.vehicles.values().map(|v| v.len()).sum();

    roads
        .nodes
        .iter()
        .filter(|n| n.incoming.is_empty())
        .for_each(|n| {
            // 10% chance to spawn a vehicle if below max
            if rand::random::<f32>() < 0.1 {
                if let Some(seg) = n.outgoing.first() {
                    if total_vehicles < 5 {
                        commands.spawn(Vehicle::new(*seg));
                        total_vehicles += 1;
                    }
                }
            }
        });
}
