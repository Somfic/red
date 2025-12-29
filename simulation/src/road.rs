use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::{Arena, Id};

#[derive(Resource, Default)]
pub struct Road {
    pub nodes: Arena<Node>,
    pub segments: Arena<Segment>,
}

pub struct Node {
    pub position: Vec3,
    pub outgoing: Vec<Id<Segment>>,
}

pub struct Segment {
    pub from: Id<Node>,
    pub to: Id<Node>,
}
