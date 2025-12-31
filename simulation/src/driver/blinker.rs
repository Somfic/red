use crate::{driver::Vehicle, Road};
use bevy_ecs::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blinker {
    #[default]
    None,
    Left,
    Right,
}

/// Update blinker state based on upcoming turn direction
pub fn update_blinkers(mut vehicles: Query<&mut Vehicle>, road: Res<Road>) {
    for mut vehicle in &mut vehicles {
        // Only check blinkers when approaching end of segment
        if vehicle.progress < 0.5 {
            vehicle.blinker = Blinker::None;
            continue;
        }

        // Get current and next segment
        let Some(&next_seg_id) = vehicle.route.get(1) else {
            vehicle.blinker = Blinker::None;
            continue;
        };

        let current_seg = road.segments.get(&vehicle.segment);
        let next_seg = road.segments.get(&next_seg_id);

        let current_from = road.nodes.get(&current_seg.from).position;
        let current_to = road.nodes.get(&current_seg.to).position;
        let next_to = road.nodes.get(&next_seg.to).position;

        // Calculate current direction and next direction
        let current_dir = (current_to - current_from).normalize_or_zero();
        let next_dir = (next_to - current_to).normalize_or_zero();

        // Cross product Z component tells us turn direction
        let cross = current_dir.x * next_dir.y - current_dir.y * next_dir.x;

        // Threshold to avoid blinking for slight curves
        vehicle.blinker = if cross > 0.3 {
            Blinker::Left
        } else if cross < -0.3 {
            Blinker::Right
        } else {
            Blinker::None
        };
    }
}
