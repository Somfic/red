use crate::{
    driver::{next_segment_toward, Idm, SegmentOccupancy},
    Id, Node, Road, Segment,
};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use rand::seq::{IndexedRandom, IteratorRandom};

#[derive(Component)]
pub struct Vehicle {
    pub speed: f32,
    pub segment: Id<Segment>,
    pub progress: f32,
    pub destination: Id<Node>,
    pub idm: Idm,
}

impl Vehicle {
    pub fn new(segment: Id<Segment>, destination: Id<Node>) -> Self {
        Self {
            speed: 0.0,
            segment,
            progress: 0.0,
            destination,
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
                let next_segment = next_segment_toward(&roads, segment.to, vehicle.destination);
                match next_segment {
                    Some(next) => {
                        vehicle.segment = next;
                        vehicle.progress -= 1.0;
                    }
                    None => commands.entity(entity).despawn(),
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
        .filter(|n| n.is_spawn && !n.outgoing.is_empty())
        .for_each(|n| {
            if rand::random::<f32>() < 0.1 {
                if let Some(seg) = n.outgoing.choose(&mut rand::rng()) {
                    if total_vehicles < 15 {
                        let (node, _) = roads
                            .nodes
                            .iter_with_ids()
                            .filter(|(_, node)| node.is_despawn && node.position != n.position)
                            .choose(&mut rand::rng())
                            .unwrap();

                        commands.spawn(Vehicle::new(*seg, node));
                        total_vehicles += 1;
                    }
                }
            }
        });
}
