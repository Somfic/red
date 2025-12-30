use crate::{driver::Idm, Id, Segment};
use bevy_ecs::prelude::*;

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
