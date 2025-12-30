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
    /// Returns the next occupant ahead and whether they're on the same segment
    pub fn find_next(
        &self,
        entity: Entity,
        vehicle: &Vehicle,
        road: &Road,
    ) -> Option<(&Occupant, bool)> {
        let mut segment = vehicle.segment;
        let mut max_iteration = 10;
        let mut progress = vehicle.progress;
        let mut same_segment = true;

        loop {
            let occupants = self.vehicles.get(&segment);

            if let Some(occupants) = occupants {
                // Find next car ahead, excluding self
                let next = occupants
                    .iter()
                    .find(|occ| occ.progress > progress && occ.vehicle != entity);

                if let Some(occ) = next {
                    return Some((occ, same_segment));
                }
            }

            // Look at next segment
            same_segment = false;
            progress = f32::MIN;
            max_iteration -= 1;
            if max_iteration == 0 {
                return None;
            }

            let seg = road.segments.get(&segment);
            let to_node = road.nodes.get(&seg.to);
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
