use crate::{
    driver::{next_segment_toward, GapAcceptance, Idm, SegmentOccupancy},
    Id, Node, Road, Segment,
};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use rand::seq::{IndexedRandom, IteratorRandom};

/// Typical car dimensions in meters
pub const DEFAULT_CAR_LENGTH: f32 = 4.5;
pub const DEFAULT_CAR_WIDTH: f32 = 1.8;

#[derive(Component)]
pub struct Vehicle {
    pub speed: f32,
    pub segment: Id<Segment>,
    pub progress: f32,
    pub destination: Id<Node>,
    pub route: Vec<Id<Segment>>,
    pub idm: Idm,
    pub gap: GapAcceptance,
    /// Vehicle length in meters (front to back)
    pub length: f32,
    /// Vehicle width in meters (side to side)
    pub width: f32,
}

impl Vehicle {
    pub fn new(segment: Id<Segment>, destination: Id<Node>, route: Vec<Id<Segment>>) -> Self {
        let aggression = rand::random();

        Self {
            speed: 0.0,
            segment,
            progress: 0.0,
            destination,
            route,
            idm: Idm::new(aggression),
            gap: GapAcceptance::new(aggression),
            length: DEFAULT_CAR_LENGTH,
            width: DEFAULT_CAR_WIDTH,
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

        let segment_length = segment.length;
        let progress_delta = vehicle.speed * time.delta_secs() / segment_length;

        vehicle.progress += progress_delta;

        // move to the next segment
        if vehicle.progress >= 1.0 {
            let to_node = roads.nodes.get(&segment.to);
            if to_node.outgoing.is_empty() {
                crate::log!("DESPAWN: to_node has no outgoing segments");
                commands.entity(entity).despawn();
            } else {
                let next_segment = next_segment_toward(&roads, segment.to, vehicle.destination);
                match next_segment {
                    Some((next, route)) => {
                        // Convert excess progress to distance, then to progress on new segment
                        let excess_distance = (vehicle.progress - 1.0) * segment_length;
                        let next_seg = roads.segments.get(&next);
                        let new_progress = excess_distance / next_seg.length;

                        vehicle.route = route;
                        vehicle.segment = next;
                        vehicle.progress = new_progress;
                        vehicle.gap.waiting_time = None;
                    }
                    None => {
                        crate::log!(
                            "DESPAWN: pathfinding returned None from {:?} to {:?}",
                            segment.to,
                            vehicle.destination
                        );
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

pub fn spawn_vehicles(mut commands: Commands, roads: Res<Road>, occupancy: Res<SegmentOccupancy>) {
    let mut total_vehicles: usize = occupancy.vehicles.values().map(|v| v.len()).sum();

    for (spawn_id, n) in roads
        .nodes
        .iter_with_ids()
        .filter(|(_, n)| n.is_spawn && !n.outgoing.is_empty())
    {
        if rand::random::<f32>() >= 0.1 || total_vehicles >= 40 {
            continue;
        }

        // Collect valid (destination, first_segment, route) candidates
        let candidates: Vec<_> = roads
            .nodes
            .iter_with_ids()
            .filter(|(_, node)| node.is_despawn && node.position != n.position)
            .filter_map(|(dest_id, _)| {
                next_segment_toward(&roads, spawn_id, dest_id)
                    .map(|(first_seg, route)| (dest_id, first_seg, route))
            })
            .collect();

        if let Some((dest_id, first_seg, route)) = candidates.choose(&mut rand::rng()) {
            commands.spawn(Vehicle::new(*first_seg, *dest_id, route.clone()));
            total_vehicles += 1;
        }
    }
}
