use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::{Arena, Id};

#[derive(Resource, Default)]
pub struct Road {
    pub nodes: Arena<Node>,
    pub segments: Arena<Segment>,
}

impl Road {
    pub fn add_node(&mut self, position: Vec3) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: false,
            is_despawn: false,
        })
    }

    pub fn add_spawn_node(&mut self, position: Vec3) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: true,
            is_despawn: false,
        })
    }

    pub fn add_despawn_node(&mut self, position: Vec3) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: false,
            is_despawn: true,
        })
    }

    pub fn add_edge_node(&mut self, position: Vec3) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: true,
            is_despawn: true,
        })
    }

    /// Add a segment between two nodes, automatically wiring up incoming/outgoing
    pub fn add_segment(&mut self, from: Id<Node>, to: Id<Node>, speed_limit: f32) -> Id<Segment> {
        let segment_id = self.segments.alloc(Segment {
            from,
            to,
            speed_limit,
        });

        // Wire up the connections
        self.nodes.get_mut(&from).outgoing.push(segment_id);
        self.nodes.get_mut(&to).incoming.push(segment_id);

        segment_id
    }

    /// Add a bidirectional road (two segments, one in each direction)
    pub fn add_bidirectional(
        &mut self,
        a: Id<Node>,
        b: Id<Node>,
        speed_limit: f32,
    ) -> (Id<Segment>, Id<Segment>) {
        let a_to_b = self.add_segment(a, b, speed_limit);
        let b_to_a = self.add_segment(b, a, speed_limit);
        (a_to_b, b_to_a)
    }
}

pub struct Node {
    pub position: Vec3,
    pub incoming: Vec<Id<Segment>>,
    pub outgoing: Vec<Id<Segment>>,
    pub is_spawn: bool,
    pub is_despawn: bool,
}

pub struct Segment {
    pub from: Id<Node>,
    pub to: Id<Node>,
    pub speed_limit: f32,
}
