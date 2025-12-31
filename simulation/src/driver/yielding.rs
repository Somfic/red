use bevy_ecs::entity::Entity;
use glam::Vec3;

use crate::driver::Blinker;

#[derive(Clone, Copy, Default)]
pub enum YieldResolver {
    #[default]
    RightOfWay,
}

/// Threshold for deadlock detection - if both cars waiting this long, break with entity ID
const DEADLOCK_THRESHOLD: f32 = 0.5;

impl YieldResolver {
    pub fn has_priority(
        &self,
        my_turn_type: TurnType,
        my_direction: Vec3,
        my_entity: Entity,
        my_waiting_time: f32,
        their_turn_type: TurnType,
        their_direction: Vec3,
        their_entity: Entity,
        their_waiting_time: f32,
    ) -> bool {
        match self {
            YieldResolver::RightOfWay => {
                // 1. Yield to right (highest priority rule)
                // Cross product of heading directions - but we need to know where they're COMING FROM
                // (approach direction = opposite of heading), so we flip the comparison
                let direction_cross = my_direction.cross(their_direction).z;
                if direction_cross < -0.3 {
                    return true; // they are to our left, we have priority
                } else if direction_cross > 0.3 {
                    // They are to our right - normally we yield
                    // But if we're both stuck waiting, break deadlock with entity ID
                    if my_waiting_time > DEADLOCK_THRESHOLD
                        && their_waiting_time > DEADLOCK_THRESHOLD
                    {
                        return my_entity < their_entity;
                    }
                    return false; // they have priority
                }

                // 2. Opposing/same direction: shorter turn path wins
                // In right-hand traffic: right turn (negative) < straight (0) < left turn (positive)
                // So more negative = shorter physical path = higher priority
                let my_path = my_turn_type.cross();
                let their_path = their_turn_type.cross();
                if (my_path - their_path).abs() > 0.1 {
                    return my_path < their_path;
                }

                // 3. Deterministic tiebreaker: lower entity ID wins
                my_entity < their_entity
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Directions: where the vehicle is HEADING (into intersection)
    const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    const DOWN: Vec3 = Vec3::new(0.0, -1.0, 0.0);
    const RIGHT: Vec3 = Vec3::new(1.0, 0.0, 0.0);
    const LEFT: Vec3 = Vec3::new(-1.0, 0.0, 0.0);

    // Dummy entities for testing
    fn entity_a() -> Entity {
        Entity::from_bits(1)
    }
    fn entity_b() -> Entity {
        Entity::from_bits(2)
    }

    #[test]
    fn test_yield_to_right_both_straight() {
        let resolver = YieldResolver::RightOfWay;

        // DOWN = facing south, coming from NORTH
        // LEFT = facing west, coming from EAST
        // From north's perspective facing south: east is to my LEFT
        // So north (DOWN) has priority over east (LEFT)
        assert!(resolver.has_priority(
            TurnType::Straight, DOWN, entity_a(), 0.0,
            TurnType::Straight, LEFT, entity_b(), 0.0,
        ));

        // From east's perspective facing west: north is to my RIGHT
        // So east (LEFT) yields to north (DOWN)
        assert!(!resolver.has_priority(
            TurnType::Straight, LEFT, entity_a(), 0.0,
            TurnType::Straight, DOWN, entity_b(), 0.0,
        ));
    }

    #[test]
    fn test_yield_to_right_overrides_turn_type() {
        let resolver = YieldResolver::RightOfWay;

        // DOWN = from north, LEFT = from east
        // North has priority over east (east is to north's left)
        // Even though the east car is turning, north still has priority
        assert!(resolver.has_priority(
            TurnType::Straight, DOWN, entity_a(), 0.0,
            TurnType::Right(-0.7), LEFT, entity_b(), 0.0,
        ));
    }

    #[test]
    fn test_opposing_directions_right_turn_beats_straight() {
        let resolver = YieldResolver::RightOfWay;

        // Opposing traffic (north vs south): direction_cross ≈ 0
        // Right turn has shorter path than straight
        // Car from north (heading DOWN) turning right vs car from south (heading UP) going straight
        assert!(resolver.has_priority(
            TurnType::Right(-0.7), DOWN, entity_a(), 0.0,
            TurnType::Straight, UP, entity_b(), 0.0,
        ));

        // Reverse: straight loses to right turn
        assert!(!resolver.has_priority(
            TurnType::Straight, UP, entity_a(), 0.0,
            TurnType::Right(-0.7), DOWN, entity_b(), 0.0,
        ));
    }

    #[test]
    fn test_opposing_directions_right_turn_beats_left_turn() {
        let resolver = YieldResolver::RightOfWay;

        // Both from opposing directions, one turning right, one turning left
        // Right turn (shorter path) wins
        assert!(resolver.has_priority(
            TurnType::Right(-0.7), DOWN, entity_a(), 0.0,
            TurnType::Left(0.7), UP, entity_b(), 0.0,
        ));

        assert!(!resolver.has_priority(
            TurnType::Left(0.7), UP, entity_a(), 0.0,
            TurnType::Right(-0.7), DOWN, entity_b(), 0.0,
        ));
    }

    #[test]
    fn test_entity_tiebreaker() {
        let resolver = YieldResolver::RightOfWay;

        // Same everything - lower entity ID wins
        assert!(resolver.has_priority(
            TurnType::Straight, DOWN, entity_a(), 0.0,
            TurnType::Straight, DOWN, entity_b(), 0.0,
        ));

        assert!(!resolver.has_priority(
            TurnType::Straight, DOWN, entity_b(), 0.0,
            TurnType::Straight, DOWN, entity_a(), 0.0,
        ));
    }

    #[test]
    fn test_deadlock_breaks_with_entity_id() {
        let resolver = YieldResolver::RightOfWay;

        // East would normally yield to north (north is to east's right)
        // But if both have been waiting > 0.5s, entity ID breaks the deadlock
        // entity_a (1) < entity_b (2), so entity_a wins
        assert!(resolver.has_priority(
            TurnType::Straight, LEFT, entity_a(), 1.0,  // east, waiting 1s
            TurnType::Straight, DOWN, entity_b(), 1.0,  // north, waiting 1s
        ));

        // With entity_b checking against entity_a, entity_b loses
        assert!(!resolver.has_priority(
            TurnType::Straight, LEFT, entity_b(), 1.0,
            TurnType::Straight, DOWN, entity_a(), 1.0,
        ));
    }

    #[test]
    fn test_yield_to_right_from_positions() {
        // Simulate actual direction computation from road.rs:
        // direction = (intersection_center - from_position).normalize()
        let resolver = YieldResolver::RightOfWay;
        let center = Vec3::ZERO;

        // Car from north (pos 0,10,0): direction = DOWN (facing south)
        let from_north = (center - Vec3::new(0.0, 10.0, 0.0)).normalize();
        // Car from east (pos 10,0,0): direction = LEFT (facing west)
        let from_east = (center - Vec3::new(10.0, 0.0, 0.0)).normalize();
        // Car from south (pos 0,-10,0): direction = UP (facing north)
        let from_south = (center - Vec3::new(0.0, -10.0, 0.0)).normalize();

        assert_eq!(from_north, DOWN);
        assert_eq!(from_east, LEFT);
        assert_eq!(from_south, UP);

        // North vs East: If I'm at north facing south, east is to my LEFT
        // So north has priority over east
        assert!(resolver.has_priority(
            TurnType::Straight, from_north, entity_a(), 0.0,
            TurnType::Straight, from_east, entity_b(), 0.0,
        ));

        // East vs North: If I'm at east facing west, north is to my RIGHT
        // So east yields to north
        assert!(!resolver.has_priority(
            TurnType::Straight, from_east, entity_a(), 0.0,
            TurnType::Straight, from_north, entity_b(), 0.0,
        ));

        // East vs South: If I'm at east facing west, south is to my LEFT
        // So east has priority over south
        assert!(resolver.has_priority(
            TurnType::Straight, from_east, entity_a(), 0.0,
            TurnType::Straight, from_south, entity_b(), 0.0,
        ));

        // South vs East: If I'm at south facing north, east is to my RIGHT
        // So south yields to east
        assert!(!resolver.has_priority(
            TurnType::Straight, from_south, entity_a(), 0.0,
            TurnType::Straight, from_east, entity_b(), 0.0,
        ));
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TurnType {
    Straight,
    Right(f32),
    Left(f32),
}

impl TurnType {
    pub fn cross(&self) -> f32 {
        match self {
            TurnType::Straight => 0.0,
            TurnType::Right(cross) => *cross,
            TurnType::Left(cross) => *cross,
        }
    }

    pub fn blinker(&self) -> Blinker {
        match self {
            TurnType::Straight => Blinker::None,
            TurnType::Right(cross) => {
                if cross.abs() > 0.3 {
                    Blinker::Right
                } else {
                    Blinker::None
                }
            }
            TurnType::Left(cross) => {
                if cross.abs() > 0.3 {
                    Blinker::Left
                } else {
                    Blinker::None
                }
            }
        }
    }
}
