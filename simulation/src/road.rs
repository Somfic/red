//! Road network representation.
//!
//! Units:
//! - Distance/Position: meters (m)
//! - Speed: meters per second (m/s)
//!
//! Speed reference:
//! - 30 km/h ≈ 8.3 m/s (residential)
//! - 50 km/h ≈ 13.9 m/s (urban)
//! - 80 km/h ≈ 22.2 m/s (highway)
//! - 120 km/h ≈ 33.3 m/s (motorway)

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use glam::Vec3;

use crate::{
    driver::{TurnType, YieldResolver},
    Arena, Id,
};

/// Speed limit constants in m/s
pub mod speed {
    /// 20 km/h - intersection/parking
    pub const SLOW: f32 = 5.5;
    /// 30 km/h - residential
    pub const RESIDENTIAL: f32 = 8.3;
    /// 50 km/h - urban
    pub const URBAN: f32 = 13.9;
    /// 80 km/h - rural/highway
    pub const HIGHWAY: f32 = 22.2;
}

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
            yield_resolver: None,
        })
    }

    pub fn add_intersection_node(
        &mut self,
        position: Vec3,
        yield_resolver: YieldResolver,
    ) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: false,
            is_despawn: false,
            yield_resolver: Some(yield_resolver),
        })
    }

    pub fn add_spawn_node(&mut self, position: Vec3) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: true,
            is_despawn: false,
            yield_resolver: None,
        })
    }

    pub fn add_despawn_node(&mut self, position: Vec3) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: false,
            is_despawn: true,
            yield_resolver: None,
        })
    }

    pub fn add_edge_node(&mut self, position: Vec3) -> Id<Node> {
        self.nodes.alloc(Node {
            position,
            incoming: vec![],
            outgoing: vec![],
            is_spawn: true,
            is_despawn: true,
            yield_resolver: None,
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
            turn_type: TurnType::Straight,
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
        const INTERSECTION_RADIUS: f32 = 8.0;
        const ROUNDABOUT_RADIUS: f32 = 12.0;
        const INNER_RADIUS: f32 = 7.0; // Inner circle for through/left traffic
        const RAMP_LENGTH: f32 = 15.0; // Straight section before roundabout curve
        const LANE_OFFSET: f32 = 1.75;

        struct EntryData {
            segment_id: Id<Segment>,
            position: Vec3,
            direction: Vec3, // direction vehicle is traveling (into intersection)
            angle: f32,      // angle around center (for sorting)
        }

        struct ExitData {
            segment_id: Id<Segment>,
            position: Vec3,
            direction: Vec3, // direction vehicle is traveling (out of intersection)
            angle: f32,      // angle around center (for sorting)
        }

        struct IntersectionData {
            node_id: Id<Node>,
            position: Vec3,
            entries: Vec<EntryData>,
            exits: Vec<ExitData>,
            yield_resolver: YieldResolver,
        }

        // Pass 1: collect all intersection data
        let intersection_data: Vec<IntersectionData> = self
            .nodes
            .iter_with_ids()
            .filter(|(_, node)| node.incoming.len() > 1 && node.outgoing.len() > 1)
            .map(|(intersection_id, intersection_node)| {
                let is_roundabout =
                    intersection_node.yield_resolver == Some(YieldResolver::Roundabout);
                let radius = if is_roundabout {
                    ROUNDABOUT_RADIUS
                } else {
                    INTERSECTION_RADIUS
                };
                // For roundabouts, edge nodes are further out to allow for straight ramps
                let edge_distance = if is_roundabout {
                    radius + RAMP_LENGTH
                } else {
                    radius
                };

                let entries = intersection_node
                    .incoming
                    .iter()
                    .map(|&segment_id| {
                        let segment = self.segments.get(&segment_id);
                        let from = self.nodes.get(&segment.from);
                        let direction = (intersection_node.position - from.position).normalize();
                        let perpendicular = direction.cross(Vec3::Z);
                        // Entry is on the right side of the approach road
                        let position = intersection_node.position - direction * edge_distance
                            + perpendicular * LANE_OFFSET;
                        // Angle of approach direction (where vehicle is coming FROM)
                        // Normalize -π to π to avoid -0.0 edge case causing inconsistent sorting
                        let mut angle = (-direction.y).atan2(-direction.x);
                        if angle <= -std::f32::consts::PI + 0.0001 {
                            angle = std::f32::consts::PI;
                        }
                        EntryData {
                            segment_id,
                            position,
                            direction,
                            angle,
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
                        // Exit is on the right side of the exit road (perpendicular offset)
                        // For roundabouts: this places it on the opposite side of the arm from entry
                        let position = intersection_node.position + direction * edge_distance
                            + perpendicular * LANE_OFFSET;
                        // Angle of exit direction (where vehicle is going TO)
                        // Normalize -π to π to avoid -0.0 edge case causing inconsistent sorting
                        let mut angle = direction.y.atan2(direction.x);
                        if angle <= -std::f32::consts::PI + 0.0001 {
                            angle = std::f32::consts::PI;
                        }
                        ExitData {
                            segment_id,
                            position,
                            direction,
                            angle,
                        }
                    })
                    .collect::<Vec<_>>();

                IntersectionData {
                    node_id: intersection_id,
                    position: intersection_node.position,
                    entries,
                    exits,
                    yield_resolver: intersection_node.yield_resolver.unwrap_or_default(),
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

            // 2c. Create intersection segments
            let mut intersection_incoming: Vec<Id<Segment>> = Vec::new();
            let mut intersection_outgoing: Vec<Id<Segment>> = Vec::new();
            let mut entry_directions: HashMap<Id<Segment>, Vec3> = HashMap::new();

            if data.yield_resolver == YieldResolver::Roundabout {
                // ROUNDABOUT: Arc triangles + inner circle design
                // Each arm has: on-ramp → A node → bypass to B node → off-ramp
                // Plus: A node → inner circle → B nodes for through/left turns
                let num_arms = data.entries.len();

                // Sort entries and exits by angle for proper ordering around the circle
                let mut sorted_entries: Vec<_> = data.entries.iter().enumerate().collect();
                sorted_entries.sort_by(|a, b| a.1.angle.partial_cmp(&b.1.angle).unwrap());

                let mut sorted_exits: Vec<_> = data.exits.iter().enumerate().collect();
                sorted_exits.sort_by(|a, b| a.1.angle.partial_cmp(&b.1.angle).unwrap());

                // Helper to compute arc center and clockwise flag given start point, start direction, and end point
                // Returns (center, radius, clockwise) for an arc that starts at p1 heading in dir1 and ends at p2
                let compute_arc = |p1: Vec3, dir1: Vec3, p2: Vec3| -> (Vec3, f32, bool) {
                    // Perpendicular to start direction (potential center lies on this line from p1)
                    let perp1 = Vec3::new(-dir1.y, dir1.x, 0.0);
                    // Midpoint between p1 and p2
                    let mid = (p1 + p2) / 2.0;
                    // Perpendicular bisector direction
                    let diff = p2 - p1;
                    let perp_bisector = Vec3::new(-diff.y, diff.x, 0.0).normalize();

                    // Solve: p1 + t * perp1 = mid + s * perp_bisector
                    let det = perp1.x * (-perp_bisector.y) - perp1.y * (-perp_bisector.x);
                    let center = if det.abs() < 0.0001 {
                        // Nearly parallel, use midpoint as fallback
                        mid
                    } else {
                        let t = ((mid.x - p1.x) * (-perp_bisector.y)
                            - (mid.y - p1.y) * (-perp_bisector.x))
                            / det;
                        p1 + perp1 * t
                    };

                    let radius = (p1 - center).length();

                    // Determine clockwise flag based on which side of the path the center is
                    // Right of direction = clockwise, Left = counter-clockwise
                    let right_dir = Vec3::new(dir1.y, -dir1.x, 0.0);
                    let clockwise = (center - p1).dot(right_dir) > 0.0;

                    (center, radius, clockwise)
                };

                // Create nodes for each arm:
                // - A node: on-ramp landing (45° right from entry direction, on outer circle)
                // - B node: off-ramp departure (45° left from exit direction, on outer circle)
                // - Inner node: same angle as A, but on inner circle
                let mut a_nodes: Vec<Id<Node>> = Vec::new();
                let mut b_nodes: Vec<Id<Node>> = Vec::new();
                let mut inner_nodes: Vec<Id<Node>> = Vec::new();

                for (_orig_idx, entry) in sorted_entries.iter() {
                    // A is 45° counter-clockwise from approach (for CCW flow, curve right to merge)
                    let a_angle = entry.angle + std::f32::consts::FRAC_PI_4;
                    let a_pos = data.position
                        + Vec3::new(a_angle.cos(), a_angle.sin(), 0.0) * ROUNDABOUT_RADIUS;
                    let a_node = self.add_node(a_pos);
                    a_nodes.push(a_node);

                    // Inner node at same angle as A, smaller radius
                    let inner_pos = data.position
                        + Vec3::new(a_angle.cos(), a_angle.sin(), 0.0) * INNER_RADIUS;
                    let inner_node = self.add_node(inner_pos);
                    inner_nodes.push(inner_node);
                }

                for (_orig_idx, exit) in sorted_exits.iter() {
                    // B is 45° clockwise from exit direction (for CCW flow, curve right to exit)
                    let b_angle = exit.angle - std::f32::consts::FRAC_PI_4;
                    let b_pos = data.position
                        + Vec3::new(b_angle.cos(), b_angle.sin(), 0.0) * ROUNDABOUT_RADIUS;
                    let b_node = self.add_node(b_pos);
                    b_nodes.push(b_node);
                }

                // Create on-ramp arcs: edge → A (curves 45° right)
                for (sorted_idx, (orig_idx, entry)) in sorted_entries.iter().enumerate() {
                    let entry_edge_id = entry_node_ids[*orig_idx];
                    let a_node = a_nodes[sorted_idx];

                    let edge_pos = self.nodes.get(&entry_edge_id).position;
                    let a_pos = self.nodes.get(&a_node).position;

                    let (arc_center, arc_radius, clockwise) =
                        compute_arc(edge_pos, entry.direction, a_pos);

                    let geometry = SegmentGeometry::Curved {
                        center: arc_center,
                        radius: arc_radius,
                        clockwise,
                    };
                    let length = geometry.length(edge_pos, a_pos);

                    let segment_id = self.segments.alloc(Segment {
                        from: entry_edge_id,
                        to: a_node,
                        speed_limit: speed::SLOW,
                        geometry,
                        turn_type: TurnType::RoundaboutEntry,
                        length,
                    });

                    entry_directions.insert(segment_id, entry.direction);

                    self.nodes.get_mut(&entry_edge_id).outgoing.push(segment_id);
                    self.nodes.get_mut(&a_node).incoming.push(segment_id);

                    intersection_incoming.push(segment_id);
                }

                // Create bypass arcs: A → next B (short arc for right turns, counter-clockwise flow)
                // In right-hand traffic, roundabout flows counter-clockwise (increasing angle)
                // So A[i] connects to B[(i+1) % num_arms] for right turn bypass
                for sorted_idx in 0..num_arms {
                    let a_node = a_nodes[sorted_idx];
                    let next_b_node = b_nodes[(sorted_idx + 1) % num_arms];

                    let a_pos = self.nodes.get(&a_node).position;
                    let b_pos = self.nodes.get(&next_b_node).position;

                    let geometry = SegmentGeometry::Curved {
                        center: data.position,
                        radius: ROUNDABOUT_RADIUS,
                        clockwise: false, // counter-clockwise flow
                    };
                    let length = geometry.length(a_pos, b_pos);

                    let segment_id = self.segments.alloc(Segment {
                        from: a_node,
                        to: next_b_node,
                        speed_limit: speed::SLOW,
                        geometry,
                        turn_type: TurnType::RoundaboutCircle,
                        length,
                    });

                    // Tangent for counter-clockwise: 90° counter-clockwise from outward
                    let from_dir = (a_pos - data.position).normalize();
                    let tangent = Vec3::new(-from_dir.y, from_dir.x, 0.0);
                    entry_directions.insert(segment_id, tangent);

                    self.nodes.get_mut(&a_node).outgoing.push(segment_id);
                    self.nodes.get_mut(&next_b_node).incoming.push(segment_id);

                    intersection_incoming.push(segment_id);
                    intersection_outgoing.push(segment_id);
                }

                // Create off-ramp arcs: B → edge (curves out)
                for (sorted_idx, (orig_idx, _exit)) in sorted_exits.iter().enumerate() {
                    let exit_edge_id = exit_node_ids[*orig_idx];
                    let b_node = b_nodes[sorted_idx];

                    let b_pos = self.nodes.get(&b_node).position;
                    let edge_pos = self.nodes.get(&exit_edge_id).position;

                    // Tangent at B pointing counter-clockwise around circle
                    let from_dir = (b_pos - data.position).normalize();
                    let tangent_at_b = Vec3::new(-from_dir.y, from_dir.x, 0.0);

                    let (arc_center, arc_radius, clockwise) =
                        compute_arc(b_pos, tangent_at_b, edge_pos);

                    let geometry = SegmentGeometry::Curved {
                        center: arc_center,
                        radius: arc_radius,
                        clockwise,
                    };
                    let length = geometry.length(b_pos, edge_pos);

                    let segment_id = self.segments.alloc(Segment {
                        from: b_node,
                        to: exit_edge_id,
                        speed_limit: speed::SLOW,
                        geometry,
                        turn_type: TurnType::RoundaboutExit,
                        length,
                    });

                    entry_directions.insert(segment_id, tangent_at_b);

                    self.nodes.get_mut(&b_node).outgoing.push(segment_id);
                    self.nodes.get_mut(&exit_edge_id).incoming.push(segment_id);

                    intersection_outgoing.push(segment_id);
                }

                // Create A → inner connections (curve inward from outer A to inner node)
                for sorted_idx in 0..num_arms {
                    let a_node = a_nodes[sorted_idx];
                    let inner_node = inner_nodes[sorted_idx];

                    let a_pos = self.nodes.get(&a_node).position;
                    let inner_pos = self.nodes.get(&inner_node).position;

                    // Simple straight segment from A to inner (they're radially aligned)
                    let geometry = SegmentGeometry::Straight;
                    let length = a_pos.distance(inner_pos);

                    let segment_id = self.segments.alloc(Segment {
                        from: a_node,
                        to: inner_node,
                        speed_limit: speed::SLOW,
                        geometry,
                        turn_type: TurnType::RoundaboutEntry,
                        length,
                    });

                    // CCW tangent direction
                    let from_dir = (a_pos - data.position).normalize();
                    let tangent = Vec3::new(-from_dir.y, from_dir.x, 0.0);
                    entry_directions.insert(segment_id, tangent);

                    self.nodes.get_mut(&a_node).outgoing.push(segment_id);
                    self.nodes.get_mut(&inner_node).incoming.push(segment_id);

                    intersection_incoming.push(segment_id);
                }

                // Create inner circle segments (counter-clockwise for right-hand traffic)
                for i in 0..num_arms {
                    let from_node = inner_nodes[i];
                    let to_node = inner_nodes[(i + 1) % num_arms]; // counter-clockwise = increasing index

                    let from_pos = self.nodes.get(&from_node).position;
                    let to_pos = self.nodes.get(&to_node).position;

                    let geometry = SegmentGeometry::Curved {
                        center: data.position,
                        radius: INNER_RADIUS,
                        clockwise: false, // counter-clockwise
                    };
                    let length = geometry.length(from_pos, to_pos);

                    let segment_id = self.segments.alloc(Segment {
                        from: from_node,
                        to: to_node,
                        speed_limit: speed::SLOW,
                        geometry,
                        turn_type: TurnType::RoundaboutCircle,
                        length,
                    });

                    // Tangent for counter-clockwise: 90° counter-clockwise from outward
                    let from_dir = (from_pos - data.position).normalize();
                    let tangent = Vec3::new(-from_dir.y, from_dir.x, 0.0);
                    entry_directions.insert(segment_id, tangent);

                    self.nodes.get_mut(&from_node).outgoing.push(segment_id);
                    self.nodes.get_mut(&to_node).incoming.push(segment_id);

                    intersection_incoming.push(segment_id);
                    intersection_outgoing.push(segment_id);
                }

                // Create inner → B connections (from each inner node to NEXT arm's B node in CCW order)
                for i in 0..num_arms {
                    let inner_node = inner_nodes[i];
                    let next_b = b_nodes[(i + 1) % num_arms]; // next B in counter-clockwise order

                    let inner_pos = self.nodes.get(&inner_node).position;
                    let b_pos = self.nodes.get(&next_b).position;

                    // Curved arc from inner circle out to B node (CCW tangent)
                    let from_dir = (inner_pos - data.position).normalize();
                    let tangent = Vec3::new(-from_dir.y, from_dir.x, 0.0);

                    let (arc_center, arc_radius, clockwise) =
                        compute_arc(inner_pos, tangent, b_pos);

                    let geometry = SegmentGeometry::Curved {
                        center: arc_center,
                        radius: arc_radius,
                        clockwise,
                    };
                    let length = geometry.length(inner_pos, b_pos);

                    let segment_id = self.segments.alloc(Segment {
                        from: inner_node,
                        to: next_b,
                        speed_limit: speed::SLOW,
                        geometry,
                        turn_type: TurnType::RoundaboutExit,
                        length,
                    });

                    entry_directions.insert(segment_id, tangent);

                    self.nodes.get_mut(&inner_node).outgoing.push(segment_id);
                    self.nodes.get_mut(&next_b).incoming.push(segment_id);

                    intersection_outgoing.push(segment_id);
                }
            } else {
                // REGULAR INTERSECTION: Create entry -> exit pairs
                for (entry_idx, entry) in data.entries.iter().enumerate() {
                    for (exit_idx, exit) in data.exits.iter().enumerate() {
                        let entry_node_id = entry_node_ids[entry_idx];
                        let exit_node_id = exit_node_ids[exit_idx];

                        // Check if this is a U-turn (directions are opposite)
                        let dot = entry.direction.dot(exit.direction);
                        if dot < -0.9 {
                            continue; // Skip U-turns
                        }

                        let cross = entry.direction.cross(exit.direction);

                        // Determine geometry: straight-through or turn
                        let geometry = if dot > 0.95 {
                            // Straight through
                            SegmentGeometry::Straight
                        } else {
                            // Turn - calculate arc
                            let clockwise = cross.z < 0.0; // cross.z < 0 = right turn (CW)

                            // Arc center is where perpendiculars from entry and exit intersect
                            let sign = if clockwise { 1.0 } else { -1.0 };
                            let entry_perp = entry.direction.cross(Vec3::Z) * sign;
                            let exit_perp = exit.direction.cross(Vec3::Z) * sign;

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

                        let entry_pos = self.nodes.get(&entry_node_id).position;
                        let exit_pos = self.nodes.get(&exit_node_id).position;
                        let length = geometry.length(entry_pos, exit_pos);

                        let turn_type = if dot > 0.95 {
                            TurnType::Straight
                        } else if cross.z < 0.0 {
                            TurnType::Right(cross.z.abs())
                        } else {
                            TurnType::Left(cross.z)
                        };

                        let segment_id = self.segments.alloc(Segment {
                            from: entry_node_id,
                            to: exit_node_id,
                            speed_limit: speed::SLOW,
                            geometry,
                            turn_type,
                            length,
                        });
                        entry_directions.insert(segment_id, entry.direction);

                        self.nodes.get_mut(&entry_node_id).outgoing.push(segment_id);
                        self.nodes.get_mut(&exit_node_id).incoming.push(segment_id);

                        intersection_incoming.push(segment_id);
                        intersection_outgoing.push(segment_id);
                    }
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
                conflicts: HashMap::new(),
                entry_directions,
                yield_resolver: self
                    .nodes
                    .get(&data.node_id)
                    .yield_resolver
                    .unwrap_or_default(),
                arrival_counter: 0,
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
                    yield_resolver: None,
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
                    yield_resolver: None,
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

            // Wire up node connections (only for newly created nodes, not reused edge nodes)
            if !from_is_edge {
                self.nodes.get_mut(&new_from).outgoing.push(seg_id);
            }
            if !to_is_edge {
                self.nodes.get_mut(&new_to).incoming.push(seg_id);
            }
        }

        // compute conflicts
        for intersection in self.intersections.iter_mut() {
            let is_roundabout = intersection.yield_resolver == YieldResolver::Roundabout;

            for (i, &seg_a_id) in intersection.incoming.iter().enumerate() {
                let seg_a = self.segments.get(&seg_a_id);
                let from_a = self.nodes.get(&seg_a.from).position;
                let to_a = self.nodes.get(&seg_a.to).position;

                for &seg_b_id in intersection.incoming.iter().skip(i + 1) {
                    let seg_b = self.segments.get(&seg_b_id);
                    let from_b = self.nodes.get(&seg_b.from).position;
                    let to_b = self.nodes.get(&seg_b.to).position;

                    let conflicts = if is_roundabout {
                        // Roundabout conflict rules:
                        // - Entry conflicts with circle segment ONLY if they merge at the same node
                        // - Circle segments don't conflict with each other (same direction)
                        // - Exit segments don't conflict with anything (diverging)
                        let a_is_entry = seg_a.turn_type == TurnType::RoundaboutEntry;
                        let b_is_entry = seg_b.turn_type == TurnType::RoundaboutEntry;
                        let a_is_circle = seg_a.turn_type == TurnType::RoundaboutCircle;
                        let b_is_circle = seg_b.turn_type == TurnType::RoundaboutCircle;

                        // Entry vs circle: only conflict if they end at the same circle node
                        if a_is_entry && b_is_circle {
                            seg_a.to == seg_b.to // Entry merges where circle segment ends
                        } else if b_is_entry && a_is_circle {
                            seg_b.to == seg_a.to // Entry merges where circle segment ends
                        } else {
                            false // Circle-circle or entry-entry don't conflict
                        }
                    } else {
                        do_segments_conflict(seg_a, seg_b, from_a, to_a, from_b, to_b)
                    };

                    if conflicts {
                        intersection
                            .conflicts
                            .entry(seg_a_id)
                            .or_default()
                            .push(seg_b_id);
                        intersection
                            .conflicts
                            .entry(seg_b_id)
                            .or_default()
                            .push(seg_a_id);
                    }
                }
            }
        }

        // Debug: print graph structure
        crate::log!("=== FINALIZE COMPLETE ===");
        crate::log!("Nodes:");
        for (id, node) in self.nodes.iter_with_ids() {
            crate::log!(
                "  {:?}: pos=({:.1}, {:.1}), in={:?}, out={:?}, spawn={}, despawn={}",
                id,
                node.position.x,
                node.position.y,
                node.incoming,
                node.outgoing,
                node.is_spawn,
                node.is_despawn
            );
        }
        crate::log!("Segments:");
        for (id, seg) in self.segments.iter_with_ids() {
            crate::log!(
                "  {:?}: {:?} -> {:?}, turn={:?}",
                id,
                seg.from,
                seg.to,
                seg.turn_type
            );
        }

        for intersection in self.intersections.iter() {
            crate::log!("Conflicts: {:?}", intersection.conflicts);
        }
    }
}

pub struct Node {
    pub position: Vec3,
    pub incoming: Vec<Id<Segment>>,
    pub outgoing: Vec<Id<Segment>>,
    pub is_spawn: bool,
    pub is_despawn: bool,
    pub yield_resolver: Option<YieldResolver>,
}

pub struct Segment {
    pub from: Id<Node>,
    pub to: Id<Node>,
    pub speed_limit: f32,
    pub geometry: SegmentGeometry,
    pub turn_type: TurnType,
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
                } else if angle_diff < 0.0 {
                    angle_diff += std::f32::consts::TAU;
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
    pub conflicts: HashMap<Id<Segment>, Vec<Id<Segment>>>,
    pub yield_resolver: YieldResolver,
    pub entry_directions: HashMap<Id<Segment>, Vec3>,
    /// Counter for FIFO arrival order at this intersection
    pub arrival_counter: u32,
}

fn do_segments_conflict(
    a: &Segment,
    b: &Segment,
    from_a: Vec3,
    to_a: Vec3,
    from_b: Vec3,
    to_b: Vec3,
) -> bool {
    const POINTS: usize = 10;

    let a_points = (0..=POINTS)
        .map(|i| {
            let t = i as f32 / POINTS as f32;
            a.geometry.position_at(from_a, to_a, t)
        })
        .collect::<Vec<_>>();

    let b_points = (0..=POINTS)
        .map(|i| {
            let t = i as f32 / POINTS as f32;
            b.geometry.position_at(from_b, to_b, t)
        })
        .collect::<Vec<_>>();

    for p_a in &a_points {
        for p_b in &b_points {
            if p_a.distance(*p_b) < 2.0 {
                return true;
            }
        }
    }

    false
}
