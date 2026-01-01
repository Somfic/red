//! Gap acceptance model for intersection behavior.
//!
//! Units:
//! - Time/gaps: seconds (s)
//! - Distance: meters (m)
//! - Speed: meters per second (m/s)

use bevy_ecs::prelude::*;
use bevy_time::Time;

use crate::{
    driver::{TurnType, Vehicle},
    Road,
};

/// Minimum physical distance (meters) to approaching vehicle before yielding
const MIN_SAFE_DISTANCE: f32 = 3.0;

pub struct GapAcceptance {
    pub min_gap: f32,
    pub waiting_time: Option<f32>,
    /// Set when deadlock detection grants priority - skip gap checks until segment transition
    pub cleared_to_go: bool,
    /// Arrival order at intersection for FIFO deadlock resolution
    pub arrival_order: Option<u32>,
}

impl GapAcceptance {
    pub fn new(aggression: f32) -> Self {
        Self {
            min_gap: blend(1.5, 1.0, aggression, 0.2),
            waiting_time: None,
            cleared_to_go: false,
            arrival_order: None,
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
    mut road: ResMut<Road>,
) {
    // Phase 1: Assign arrival orders to vehicles entering waiting zone (FIFO ordering)
    for (_entity, mut vehicle) in vehicles.iter_mut().filter(|(_, v)| v.progress > 0.5) {
        if vehicle.gap.arrival_order.is_some() {
            continue; // Already has an arrival order
        }

        let next_segment = match vehicle.route.get(1) {
            Some(seg) => *seg,
            None => continue,
        };

        // Find the intersection this vehicle is approaching and assign arrival order
        for intersection in road.intersections.iter_mut() {
            if intersection.incoming.contains(&next_segment) {
                vehicle.gap.arrival_order = Some(intersection.arrival_counter);
                intersection.arrival_counter += 1;
                break;
            }
        }
    }

    // Phase 2: Collect info about all vehicles approaching intersections
    // Tuple: (entity, segment, next_segment, progress, speed, length, waiting_time, arrival_order)
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
                v.gap.arrival_order.unwrap_or(u32::MAX),
            )
        })
        .collect();

    // Phase 3: Gap acceptance checks
    for (entity, mut vehicle) in vehicles.iter_mut().filter(|(_, v)| v.progress > 0.5) {
        let next_segment = match vehicle.route.get(1) {
            Some(seg) => seg,
            None => continue,
        };

        let critical_time = vehicle.gap.min_gap;
        let my_arrival_order = vehicle.gap.arrival_order.unwrap_or(u32::MAX);

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
                    other_arrival_order,
                ) in &vehicle_info
                {
                    if other_entity == entity {
                        continue; // skip self
                    }

                    // Safety check 1: Yield to vehicles already IN the intersection on conflicting segments
                    // For roundabouts: circle traffic has priority over entry traffic, even if entry is already in
                    if conflicts.contains(&other_seg) {
                        let my_turn = road.segments.get(next_segment).turn_type;
                        let other_turn = road.segments.get(&other_seg).turn_type;

                        // For roundabouts: circle traffic doesn't yield to entry traffic
                        let dominated = match intersection.yield_resolver {
                            crate::driver::YieldResolver::Roundabout => {
                                // Only yield if I'm entering and they're in the circle
                                my_turn == TurnType::RoundaboutEntry
                                    && other_turn == TurnType::RoundaboutCircle
                            }
                            _ => true, // For regular intersections, always yield to vehicles already in
                        };

                        if dominated {
                            actual_gap = 0.0;
                            break;
                        }
                    }

                    // Check vehicles approaching conflicting segments
                    if let Some(other_next_seg) = other_next {
                        if conflicts.contains(&other_next_seg) {
                            // Priority check using arrival order for FIFO deadlock resolution
                            let my_turn = road.segments.get(next_segment).turn_type;
                            let my_dir = *intersection.entry_directions.get(next_segment).unwrap();

                            let their_turn = road.segments.get(&other_next_seg).turn_type;
                            let their_dir =
                                *intersection.entry_directions.get(&other_next_seg).unwrap();

                            if intersection.yield_resolver.has_priority(
                                my_turn,
                                my_dir,
                                my_arrival_order,
                                vehicle.gap.waiting_time.unwrap_or(0.0),
                                their_turn,
                                their_dir,
                                other_arrival_order,
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
