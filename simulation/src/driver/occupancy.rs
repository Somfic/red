use crate::{driver::Vehicle, Id, Road, Segment};
use bevy_ecs::prelude::*;
use std::collections::HashMap;

pub struct Occupant {
    pub progress: f32,
    pub vehicle: Entity,
    pub speed: f32,
    pub segment: Id<Segment>,
}

#[derive(Resource, Default)]
pub struct SegmentOccupancy {
    pub vehicles: HashMap<Id<Segment>, Vec<Occupant>>,
}

impl SegmentOccupancy {
    /// Returns the next occupant ahead and the distance to them in world units
    pub fn find_next(
        &self,
        entity: Entity,
        vehicle: &Vehicle,
        road: &Road,
    ) -> Option<(&Occupant, f32)> {
        let mut segment = vehicle.segment;
        let mut max_iteration = 10;
        let mut progress = vehicle.progress;
        let mut accumulated_distance: f32 = 0.0;
        let mut first_segment = true;

        loop {
            let seg_data = road.segments.get(&segment);
            let from = road.nodes.get(&seg_data.from);
            let to = road.nodes.get(&seg_data.to);
            let segment_length = from.position.distance(to.position);

            let occupants = self.vehicles.get(&segment);

            if let Some(occupants) = occupants {
                // Find next car ahead, excluding self
                let next = occupants
                    .iter()
                    .find(|occ| occ.progress > progress && occ.vehicle != entity);

                if let Some(occ) = next {
                    // Calculate distance to this occupant
                    let distance = if first_segment {
                        // Same segment: simple progress difference
                        (occ.progress - vehicle.progress) * segment_length
                    } else {
                        // Different segment: accumulated + their progress
                        accumulated_distance + occ.progress * segment_length
                    };
                    return Some((occ, distance));
                }
            }

            // Add remaining distance on this segment before moving to next
            if first_segment {
                accumulated_distance += (1.0 - vehicle.progress) * segment_length;
                first_segment = false;
            } else {
                accumulated_distance += segment_length;
            }

            // Look at next segment
            progress = f32::MIN;
            max_iteration -= 1;
            if max_iteration == 0 {
                return None;
            }

            let to_node = road.nodes.get(&seg_data.to);
            if to_node.outgoing.is_empty() {
                return None;
            } else {
                segment = *to_node.outgoing.first().unwrap();
            }
        }
    }
}

pub fn update_occupancy(
    mut occupancy: ResMut<SegmentOccupancy>,
    vehicles: Query<(Entity, &Vehicle)>,
) {
    occupancy.vehicles.clear();

    for (entity, vehicle) in &vehicles {
        let entry = occupancy.vehicles.entry(vehicle.segment).or_default();
        entry.push(Occupant {
            progress: vehicle.progress,
            vehicle: entity,
            speed: vehicle.speed,
            segment: vehicle.segment,
        });
    }

    // Sort vehicles on each segment by progress
    for occupants in occupancy.vehicles.values_mut() {
        occupants.sort_by(|a, b| a.progress.partial_cmp(&b.progress).unwrap());
    }
}
