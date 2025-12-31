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
    pub min_gap: f32,
    pub waiting_time: Option<f32>,
    /// Set when deadlock detection grants priority - skip gap checks until segment transition
    pub cleared_to_go: bool,
}

impl GapAcceptance {
    pub fn new(aggression: f32) -> Self {
        Self {
            min_gap: blend(1.5, 1.0, aggression, 0.2),
            waiting_time: None,
            cleared_to_go: false,
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
    // Tuple: (entity, segment, next_segment, progress, speed, length, waiting_time)
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
                v.gap.waiting_time.unwrap_or(0.0),
            )
        })
        .collect();

    for (entity, mut vehicle) in vehicles.iter_mut().filter(|(_, v)| v.progress > 0.5) {
        let next_segment = match vehicle.route.get(1) {
            Some(seg) => seg,
            None => continue,
        };

        let critical_time = vehicle.gap.min_gap;

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
                    other_waiting_time,
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

                    // if other_speed < 0.5 {
                    //     continue; // skip slow/stopped vehicles for approach detection
                    // }

                    // Check vehicles approaching conflicting segments
                    if let Some(other_next_seg) = other_next {
                        if conflicts.contains(&other_next_seg) {
                            // Priority check
                            let my_turn = road.segments.get(next_segment).turn_type;
                            let my_dir = *intersection.entry_directions.get(next_segment).unwrap();

                            let their_turn = road.segments.get(&other_next_seg).turn_type;
                            let their_dir =
                                *intersection.entry_directions.get(&other_next_seg).unwrap();

                            if intersection.yield_resolver.has_priority(
                                my_turn,
                                my_dir,
                                entity,
                                vehicle.gap.waiting_time.unwrap_or(0.0),
                                their_turn,
                                their_dir,
                                other_entity,
                                other_waiting_time,
                            ) {
                                continue; // I have priority, don't yield to this vehicle
                            }

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
            // Must wait - accumulate waiting time for deadlock detection
            let current = vehicle.gap.waiting_time.unwrap_or(0.0);
            vehicle.gap.waiting_time = Some(current + time.delta_secs());
            vehicle.gap.cleared_to_go = false;
        } else {
            // Gap is acceptable - tell IDM we can go
            // Keep waiting_time for deadlock detection (cleared on segment transition)
            vehicle.gap.cleared_to_go = true;
        }
    }
}
