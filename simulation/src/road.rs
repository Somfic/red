use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::{Arena, Id};

#[derive(Resource, Default)]
pub struct Road {
    pub nodes: Arena<Node>,
    pub segments: Arena<Segment>,
    pub intersections: Arena<Intersection>,
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
        let from_pos = self.nodes.get(&from).position;
        let to_pos = self.nodes.get(&to).position;
        let geometry = SegmentGeometry::Straight;
        let length = geometry.length(from_pos, to_pos);

        let segment_id = self.segments.alloc(Segment {
            from,
            to,
            speed_limit,
            geometry,
            length,
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

    pub fn finalize(&mut self) {
        const INTERSECTION_RADIUS: f32 = 3.0;
        const LANE_OFFSET: f32 = 0.5;

        struct EntryData {
            segment_id: Id<Segment>,
            position: Vec3,
            direction: Vec3, // direction vehicle is traveling (into intersection)
        }

        struct ExitData {
            segment_id: Id<Segment>,
            position: Vec3,
            direction: Vec3, // direction vehicle is traveling (out of intersection)
        }

        struct IntersectionData {
            node_id: Id<Node>,
            position: Vec3,
            entries: Vec<EntryData>,
            exits: Vec<ExitData>,
        }

        // Pass 1: collect all intersection data
        let intersection_data: Vec<IntersectionData> = self
            .nodes
            .iter_with_ids()
            .filter(|(_, node)| node.incoming.len() > 1 && node.outgoing.len() > 1)
            .map(|(intersection_id, intersection_node)| {
                let entries = intersection_node
                    .incoming
                    .iter()
                    .map(|&segment_id| {
                        let segment = self.segments.get(&segment_id);
                        let from = self.nodes.get(&segment.from);
                        let direction = (intersection_node.position - from.position).normalize();
                        let perpendicular = direction.cross(Vec3::Z);
                        let position = intersection_node.position - direction * INTERSECTION_RADIUS
                            + perpendicular * LANE_OFFSET;
                        EntryData {
                            segment_id,
                            position,
                            direction,
                        }
                    })
                    .collect::<Vec<_>>();

                let exits = intersection_node
                    .outgoing
                    .iter()
                    .map(|&segment_id| {
                        let segment = self.segments.get(&segment_id);
                        let to = self.nodes.get(&segment.to);
                        let direction = (to.position - intersection_node.position).normalize();
                        let perpendicular = direction.cross(Vec3::Z);
                        let position = intersection_node.position
                            + direction * INTERSECTION_RADIUS
                            + perpendicular * LANE_OFFSET;
                        ExitData {
                            segment_id,
                            position,
                            direction,
                        }
                    })
                    .collect::<Vec<_>>();

                IntersectionData {
                    node_id: intersection_id,
                    position: intersection_node.position,
                    entries,
                    exits,
                }
            })
            .collect();

        // Pass 2: create edge nodes, rewire segments, create intersection segments
        for data in intersection_data {
            let mut entry_node_ids: Vec<Id<Node>> = Vec::new();
            let mut exit_node_ids: Vec<Id<Node>> = Vec::new();

            // 2a. Create entry edge nodes and rewire incoming segments
            for entry in &data.entries {
                let entry_node_id = self.add_node(entry.position);
                entry_node_ids.push(entry_node_id);

                // Rewire: segment now ends at entry node instead of intersection
                let segment = self.segments.get_mut(&entry.segment_id);
                segment.to = entry_node_id;

                // Update node connections
                self.nodes
                    .get_mut(&entry_node_id)
                    .incoming
                    .push(entry.segment_id);
            }

            // 2b. Create exit edge nodes and rewire outgoing segments
            for exit in &data.exits {
                let exit_node_id = self.add_node(exit.position);
                exit_node_ids.push(exit_node_id);

                // Rewire: segment now starts at exit node instead of intersection
                let segment = self.segments.get_mut(&exit.segment_id);
                segment.from = exit_node_id;

                // Update node connections
                self.nodes
                    .get_mut(&exit_node_id)
                    .outgoing
                    .push(exit.segment_id);
            }

            // 2c. Create intersection segments (entry -> exit pairs)
            let mut intersection_incoming: Vec<Id<Segment>> = Vec::new();
            let mut intersection_outgoing: Vec<Id<Segment>> = Vec::new();

            for (entry_idx, entry) in data.entries.iter().enumerate() {
                for (exit_idx, exit) in data.exits.iter().enumerate() {
                    let entry_node_id = entry_node_ids[entry_idx];
                    let exit_node_id = exit_node_ids[exit_idx];

                    // Check if this is a U-turn (directions are opposite)
                    let dot = entry.direction.dot(exit.direction);
                    if dot < -0.9 {
                        continue; // Skip U-turns
                    }

                    // Determine geometry: straight-through or turn
                    let geometry = if dot > 0.9 {
                        // Straight through
                        SegmentGeometry::Straight
                    } else {
                        // Turn - calculate arc
                        let cross = entry.direction.cross(exit.direction);
                        let clockwise = cross.z < 0.0; // cross.z < 0 = right turn (CW)

                        // Arc center is where perpendiculars from entry and exit intersect
                        // For right turn: perpendicular to the right
                        // For left turn: perpendicular to the left
                        let sign = if clockwise { 1.0 } else { -1.0 };
                        let entry_perp = entry.direction.cross(Vec3::Z) * sign;
                        let exit_perp = exit.direction.cross(Vec3::Z) * sign;

                        // Find intersection of two lines:
                        // Line 1: entry.position + t * entry_perp
                        // Line 2: exit.position + s * exit_perp
                        // Solve: entry.position + t * entry_perp = exit.position + s * exit_perp
                        let d = entry_perp.x * exit_perp.y - entry_perp.y * exit_perp.x;
                        let t = if d.abs() > 0.001 {
                            ((exit.position.x - entry.position.x) * exit_perp.y
                                - (exit.position.y - entry.position.y) * exit_perp.x)
                                / d
                        } else {
                            1.0 // fallback for parallel lines
                        };

                        let center = entry.position + entry_perp * t;
                        let radius = (entry.position - center).length();

                        SegmentGeometry::Curved {
                            center,
                            radius,
                            clockwise,
                        }
                    };

                    // Create the intersection segment
                    let entry_pos = self.nodes.get(&entry_node_id).position;
                    let exit_pos = self.nodes.get(&exit_node_id).position;
                    let length = geometry.length(entry_pos, exit_pos);

                    let segment_id = self.segments.alloc(Segment {
                        from: entry_node_id,
                        to: exit_node_id,
                        speed_limit: 5.0, // intersection speed limit
                        geometry,
                        length,
                    });

                    // Wire up connections
                    self.nodes.get_mut(&entry_node_id).outgoing.push(segment_id);
                    self.nodes.get_mut(&exit_node_id).incoming.push(segment_id);

                    intersection_incoming.push(segment_id);
                    intersection_outgoing.push(segment_id);
                }
            }

            // 2d. Create Intersection record
            let mut all_edge_nodes = entry_node_ids.clone();
            all_edge_nodes.extend(exit_node_ids);

            self.intersections.alloc(Intersection {
                position: data.position,
                incoming: intersection_incoming,
                outgoing: intersection_outgoing,
                edge_nodes: all_edge_nodes,
            });

            // Clear the original intersection node's connections (it's no longer used for routing)
            let original_node = self.nodes.get_mut(&data.node_id);
            original_node.incoming.clear();
            original_node.outgoing.clear();
        }

        // Pass 3: Offset remaining straight segment endpoints
        // Collect all edge node IDs (these already have offset baked in)
        let edge_node_ids: std::collections::HashSet<Id<Node>> = self
            .intersections
            .iter()
            .flat_map(|i| i.edge_nodes.iter().copied())
            .collect();

        // Collect segment data for offsetting (to avoid borrow issues)
        // Must capture node flags NOW before we start modifying them
        // Note: spawn nodes are segment sources, despawn nodes are segment destinations
        let segments_to_offset: Vec<_> = self
            .segments
            .iter_with_ids()
            .filter(|(_, seg)| matches!(seg.geometry, SegmentGeometry::Straight))
            .map(|(seg_id, seg)| {
                let from_node = self.nodes.get(&seg.from);
                let to_node = self.nodes.get(&seg.to);
                let direction = (to_node.position - from_node.position).normalize();
                let perpendicular = direction.cross(Vec3::Z);

                (
                    seg_id,
                    seg.from,
                    seg.to,
                    from_node.position + perpendicular * LANE_OFFSET,
                    to_node.position + perpendicular * LANE_OFFSET,
                    edge_node_ids.contains(&seg.from),
                    edge_node_ids.contains(&seg.to),
                    from_node.is_spawn, // Source can be spawn point
                    to_node.is_despawn, // Destination can be despawn point
                )
            })
            .collect();

        // Create offset nodes and rewire segments
        for (
            seg_id,
            old_from,
            old_to,
            from_offset_pos,
            to_offset_pos,
            from_is_edge,
            to_is_edge,
            from_is_spawn,
            to_is_despawn,
        ) in segments_to_offset
        {
            let new_from = if from_is_edge {
                old_from // Already offset
            } else {
                // Create offset node - source nodes can only be spawn points
                let new_node_id = self.nodes.alloc(Node {
                    position: from_offset_pos,
                    incoming: vec![],
                    outgoing: vec![],
                    is_spawn: from_is_spawn,
                    is_despawn: false,
                });
                // Clear old node's connections and flags (no longer used for routing)
                let old_node_mut = self.nodes.get_mut(&old_from);
                old_node_mut.outgoing.retain(|&id| id != seg_id);
                old_node_mut.is_spawn = false;
                old_node_mut.is_despawn = false;
                new_node_id
            };

            let new_to = if to_is_edge {
                old_to // Already offset
            } else {
                // Create offset node - destination nodes can only be despawn points
                let new_node_id = self.nodes.alloc(Node {
                    position: to_offset_pos,
                    incoming: vec![],
                    outgoing: vec![],
                    is_spawn: false,
                    is_despawn: to_is_despawn,
                });
                // Clear old node's connections and flags (no longer used for routing)
                let old_node_mut = self.nodes.get_mut(&old_to);
                old_node_mut.incoming.retain(|&id| id != seg_id);
                old_node_mut.is_spawn = false;
                old_node_mut.is_despawn = false;
                new_node_id
            };

            // Rewire segment
            let segment = self.segments.get_mut(&seg_id);
            segment.from = new_from;
            segment.to = new_to;

            // Update length based on new positions
            let from_pos = self.nodes.get(&new_from).position;
            let to_pos = self.nodes.get(&new_to).position;
            segment.length = segment.geometry.length(from_pos, to_pos);

            // Wire up node connections
            self.nodes.get_mut(&new_from).outgoing.push(seg_id);
            self.nodes.get_mut(&new_to).incoming.push(seg_id);
        }

        // Debug: print graph structure
        crate::log!("=== FINALIZE COMPLETE ===");
        crate::log!("Nodes:");
        for (id, node) in self.nodes.iter_with_ids() {
            crate::log!(
                "  {:?}: pos={:?}, in={}, out={}, spawn={}, despawn={}",
                id,
                node.position,
                node.incoming.len(),
                node.outgoing.len(),
                node.is_spawn,
                node.is_despawn
            );
        }
        crate::log!("Segments:");
        for (id, seg) in self.segments.iter_with_ids() {
            crate::log!("  {:?}: {:?} -> {:?}", id, seg.from, seg.to);
        }
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
    pub geometry: SegmentGeometry,
    pub length: f32,
}

pub enum SegmentGeometry {
    Straight,
    Curved {
        center: Vec3,
        radius: f32,
        clockwise: bool,
    },
}

impl SegmentGeometry {
    /// Calculate the length of the path (arc length for curves, straight distance for lines)
    pub fn length(&self, from: Vec3, to: Vec3) -> f32 {
        match self {
            SegmentGeometry::Straight => from.distance(to),
            SegmentGeometry::Curved {
                center,
                radius,
                clockwise,
            } => {
                // Calculate the angle swept by the arc
                let start_offset = from - *center;
                let end_offset = to - *center;

                let start_angle = start_offset.y.atan2(start_offset.x);
                let end_angle = end_offset.y.atan2(end_offset.x);

                let mut angle_diff = end_angle - start_angle;

                if *clockwise {
                    if angle_diff > 0.0 {
                        angle_diff -= std::f32::consts::TAU;
                    }
                } else {
                    if angle_diff < 0.0 {
                        angle_diff += std::f32::consts::TAU;
                    }
                }

                radius * angle_diff.abs()
            }
        }
    }

    /// Calculate position along a segment given progress (0.0 to 1.0)
    pub fn position_at(&self, from: Vec3, to: Vec3, progress: f32) -> Vec3 {
        // Ensure exact endpoints to avoid floating point discontinuities
        if progress <= 0.0 {
            return from;
        }
        if progress >= 1.0 {
            return to;
        }

        match self {
            SegmentGeometry::Straight => from.lerp(to, progress),
            SegmentGeometry::Curved {
                center,
                radius,
                clockwise,
            } => {
                // Calculate start and end angles
                let start_offset = from - *center;
                let end_offset = to - *center;

                let start_angle = start_offset.y.atan2(start_offset.x);
                let end_angle = end_offset.y.atan2(end_offset.x);

                // Calculate angle difference, respecting direction
                let mut angle_diff = end_angle - start_angle;

                if *clockwise {
                    // Clockwise: angle should decrease (or wrap around)
                    if angle_diff > 0.0 {
                        angle_diff -= std::f32::consts::TAU;
                    }
                } else {
                    // Counter-clockwise: angle should increase (or wrap around)
                    if angle_diff < 0.0 {
                        angle_diff += std::f32::consts::TAU;
                    }
                }

                let current_angle = start_angle + angle_diff * progress;

                Vec3::new(
                    center.x + current_angle.cos() * radius,
                    center.y + current_angle.sin() * radius,
                    from.z, // preserve Z
                )
            }
        }
    }

    /// Calculate direction (tangent) along a segment given progress (0.0 to 1.0)
    pub fn direction_at(&self, from: Vec3, to: Vec3, progress: f32) -> Vec3 {
        match self {
            SegmentGeometry::Straight => (to - from).normalize(),
            SegmentGeometry::Curved {
                center, clockwise, ..
            } => {
                let pos = self.position_at(from, to, progress);
                let radial = (pos - *center).normalize();

                // Tangent is perpendicular to radial
                // Clockwise: rotate radial -90° (right)
                // Counter-clockwise: rotate radial +90° (left)
                if *clockwise {
                    Vec3::new(radial.y, -radial.x, 0.0)
                } else {
                    Vec3::new(-radial.y, radial.x, 0.0)
                }
            }
        }
    }
}

pub struct Intersection {
    pub position: Vec3,
    pub incoming: Vec<Id<Segment>>,
    pub outgoing: Vec<Id<Segment>>,
    pub edge_nodes: Vec<Id<Node>>,
}
