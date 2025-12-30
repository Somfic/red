use bevy_ecs::{
    entity::Entity,
    query::Without,
    system::{Query, Res},
};
use bevy_time::Time;

use crate::{
    driver::{PlayerControlled, SegmentOccupancy, Vehicle},
    Road,
};

pub struct Idm {
    pub aggression: f32,
    pub desired_time_headway: f32,
    pub min_spacing: f32,
    pub max_acceleration: f32,
    pub comfortable_deceleration: f32,
}

impl Idm {
    pub fn new(aggression: f32) -> Self {
        Self {
            aggression,
            desired_time_headway: blend(1.5, 0.8, aggression, 0.2).max(0.5),
            min_spacing: blend(2.0, 1.0, aggression, 0.5).max(0.5),
            max_acceleration: blend(1.0, 3.0, aggression, 0.5).max(0.5),
            comfortable_deceleration: blend(1.5, 3.0, aggression, 0.5).max(0.5),
        }
    }

    pub fn acceleration(&self, speed_limit: f32, speed: f32, gap: f32, delta_speed: f32) -> f32 {
        let desired_speed = lerp(speed_limit * 0.8, speed_limit * 1.2, self.aggression);

        let gap = gap.max(0.01);

        let s_star = self.min_spacing
            + speed * self.desired_time_headway
            + (speed * delta_speed)
                / (2.0 * (self.max_acceleration * self.comfortable_deceleration).sqrt());

        self.max_acceleration * (1.0 - (speed / desired_speed).powi(4) - (s_star / gap).powi(2))
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

pub fn apply_idm(
    time: Res<Time>,
    mut vehicles: Query<(Entity, &mut Vehicle), Without<PlayerControlled>>,
    occupancy: Res<SegmentOccupancy>,
    road: Res<Road>,
) {
    for (entity, mut vehicle) in &mut vehicles {
        let segment = road.segments.get(&vehicle.segment);

        let next_driver = occupancy.find_next(entity, &vehicle, &road);

        let (gap, delta_speed) = if let Some((next_occupant, distance)) = next_driver {
            let delta_speed = vehicle.speed - next_occupant.speed;
            (distance, delta_speed)
        } else {
            (f32::MAX, 0.0)
        };

        let acceleration =
            vehicle
                .idm
                .acceleration(segment.speed_limit, vehicle.speed, gap, delta_speed);

        vehicle.speed = (vehicle.speed + acceleration * time.delta_secs()).max(0.0);
    }
}
