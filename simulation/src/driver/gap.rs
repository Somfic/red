//! Gap acceptance model for intersection behavior.
//!
//! Units:
//! - Time/gaps: seconds (s)
//! - Distance: meters (m)
//! - Speed: meters per second (m/s)

use bevy_ecs::prelude::*;
use bevy_time::Time;

use crate::{driver::Vehicle, Road};

/// Minimum physical distance (meters) to approaching vehicle before yielding
const MIN_SAFE_DISTANCE: f32 = 3.0;

pub struct GapAcceptance {
    pub safe_gap: f32,
    pub urgency_rate: f32,
    pub min_gap: f32,
    pub waiting_time: Option<f32>,
}

const URGENCY_RATE: f32 = 0.15;

impl GapAcceptance {
    pub fn new(aggression: f32) -> Self {
        Self {
            safe_gap: blend(3.0, 1.0, aggression, 0.5),
            urgency_rate: URGENCY_RATE,
            min_gap: blend(1.5, 1.0, aggression, 0.2),
            waiting_time: None,
        }
    }
}

fn blend(safe_value: f32, aggressive_value: f32, aggression: f32, max_random_range: f32) -> f32 {
    let random = rand::random::<f32>() * 2.0 - 1.0;
    let random = max_random_range * random;

    lerp(safe_value, aggressive_value, aggression) + random
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// TODO: Store conflict_progress in conflicts HashMap to calculate precise time-to-conflict
// Currently uses time-to-end-of-segment as approximation
pub fn apply_gap_acceptance(
    time: Res<Time>,
    mut vehicles: Query<(Entity, &mut Vehicle)>,
    road: Res<Road>,
) {
    // Collect info about all vehicles approaching intersections
    // Tuple: (entity, segment, next_segment, progress, speed, length)
    let vehicle_info: Vec<_> = vehicles
        .iter()
        .map(|(entity, v)| {
            (
                entity,
                v.segment,
                v.route.get(1).copied(),
                v.progress,
                v.speed,
                v.length,
            )
        })
        .collect();

    for (entity, mut vehicle) in vehicles.iter_mut().filter(|(_, v)| v.progress > 0.5) {
        let next_segment = match vehicle.route.get(1) {
            Some(seg) => seg,
            None => continue,
        };

        let critical_time = (vehicle.gap.safe_gap
            * f32::exp(-vehicle.gap.urgency_rate * vehicle.gap.waiting_time.unwrap_or(0.0)))
        .max(vehicle.gap.min_gap);

        let mut actual_gap = f32::MAX;

        // find intersection containing next_segment
        for intersection in road
            .intersections
            .iter()
            .filter(|i| i.incoming.contains(next_segment))
        {
            if let Some(conflicts) = intersection.conflicts.get(next_segment) {
                for &(
                    other_entity,
                    other_seg,
                    other_next,
                    other_progress,
                    other_speed,
                    other_length,
                ) in &vehicle_info
                {
                    if other_entity == entity {
                        continue; // skip self
                    }

                    // Safety check 1: Yield to vehicles already IN the intersection on conflicting segments
                    if conflicts.contains(&other_seg) {
                        // Vehicle is currently on a conflicting segment - must wait
                        actual_gap = 0.0;
                        break;
                    }

                    if other_speed < 0.5 {
                        continue; // skip slow/stopped vehicles for approach detection
                    }

                    // Check vehicles approaching conflicting segments
                    if let Some(other_next_seg) = other_next {
                        if conflicts.contains(&other_next_seg) {
                            let seg = road.segments.get(&other_seg);
                            let remaining = 1.0 - other_progress;

                            // Distance from front bumper to intersection entry
                            let distance_to_enter =
                                (remaining * seg.length - other_length / 2.0).max(0.0);

                            // Safety check 2: Minimum physical distance
                            if distance_to_enter < MIN_SAFE_DISTANCE {
                                actual_gap = 0.0;
                                break;
                            }

                            let time_to_enter = distance_to_enter / other_speed.max(0.1);
                            actual_gap = actual_gap.min(time_to_enter);
                        }
                    }
                }
            }
        }

        if actual_gap < critical_time {
            let current = vehicle.gap.waiting_time.unwrap_or(0.0);
            vehicle.gap.waiting_time = Some(current + time.delta_secs());
        } else {
            // Clear if gap is acceptable (not just if it's MAX)
            vehicle.gap.waiting_time = None;
        }
    }
}
