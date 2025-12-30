use crate::{Id, Node, Road, Segment};
use std::collections::{HashMap, VecDeque};

pub fn next_segment_toward(
    road: &Road,
    current: Id<Node>,
    destination: Id<Node>,
) -> Option<Id<Segment>> {
    if current == destination {
        return None; // arrived
    }

    let mut queue = VecDeque::<Id<Node>>::new();
    let mut came_from = HashMap::<Id<Node>, Id<Segment>>::new();

    let current_node = road.nodes.get(&current);
    for segment_id in &current_node.outgoing {
        let neighbor = road.segments.get(segment_id).to;
        queue.push_back(neighbor);
        came_from.insert(neighbor, *segment_id);
    }

    // bfs
    while let Some(node_id) = queue.pop_front() {
        if node_id == destination {
            let mut backtrack = destination;
            loop {
                let previous_id = came_from.get(&backtrack).unwrap();
                let previous = road.segments.get(previous_id);

                if previous.from == current {
                    return Some(*previous_id);
                }

                backtrack = previous.from;
            }
        }

        let node = road.nodes.get(&node_id);
        for segment_id in &node.outgoing {
            let neighbor = road.segments.get(segment_id).to;
            came_from.entry(neighbor).or_insert_with(|| {
                queue.push_back(neighbor);
                *segment_id
            });
        }
    }

    None
}
