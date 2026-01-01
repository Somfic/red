use glam::Vec3;

use crate::driver::Blinker;

#[derive(Clone, Copy, Default)]
pub enum YieldResolver {
    #[default]
    RightOfWay,
}

/// Threshold for deadlock detection - if both cars waiting this long, use arrival order
const DEADLOCK_THRESHOLD: f32 = 0.5;

impl YieldResolver {
    /// Determines if the current vehicle has priority over another vehicle.
    /// Uses arrival_order (FIFO) for deadlock resolution - earlier arrivals get priority.
    pub fn has_priority(
        &self,
        my_turn_type: TurnType,
        my_direction: Vec3,
        my_arrival_order: u32,
        my_waiting_time: f32,
        their_turn_type: TurnType,
        their_direction: Vec3,
        their_arrival_order: u32,
        their_waiting_time: f32,
    ) -> bool {
        match self {
            YieldResolver::RightOfWay => {
                // 0. FIFO queue priority: vehicles in the queue go before those not yet in queue
                // arrival_order = u32::MAX means vehicle hasn't entered waiting zone yet
                if their_arrival_order == u32::MAX && my_arrival_order != u32::MAX {
                    return true; // I'm in queue, they're not - I have priority
                }
                if my_arrival_order == u32::MAX && their_arrival_order != u32::MAX {
                    return false; // They're in queue, I'm not - they have priority
                }

                // DEADLOCK OVERRIDE: If both vehicles have been waiting a long time,
                // use pure FIFO (arrival order) to break any circular dependencies
                if my_waiting_time > DEADLOCK_THRESHOLD && their_waiting_time > DEADLOCK_THRESHOLD {
                    return my_arrival_order < their_arrival_order;
                }

                // 1. Yield to right (highest priority rule)
                // Cross product of heading directions - but we need to know where they're COMING FROM
                // (approach direction = opposite of heading), so we flip the comparison
                let direction_cross = my_direction.cross(their_direction).z;
                if direction_cross < -0.3 {
                    return true; // they are to our left, we have priority
                } else if direction_cross > 0.3 {
                    return false; // they are to our right, they have priority
                }

                // 2. Opposing/same direction: shorter turn path wins
                // In right-hand traffic: right turn (negative) < straight (0) < left turn (positive)
                // So more negative = shorter physical path = higher priority
                let my_path = my_turn_type.cross();
                let their_path = their_turn_type.cross();
                if (my_path - their_path).abs() > 0.1 {
                    return my_path < their_path;
                }

                // 3. Deterministic tiebreaker: earlier arrival wins (FIFO)
                my_arrival_order < their_arrival_order
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

    // Arrival orders for testing (lower = arrived first)
    const FIRST: u32 = 1;
    const SECOND: u32 = 2;

    #[test]
    fn test_yield_to_right_both_straight() {
        let resolver = YieldResolver::RightOfWay;

        // DOWN = facing south, coming from NORTH
        // LEFT = facing west, coming from EAST
        // From north's perspective facing south: east is to my LEFT
        // So north (DOWN) has priority over east (LEFT)
        assert!(resolver.has_priority(
            TurnType::Straight,
            DOWN,
            FIRST,
            0.0,
            TurnType::Straight,
            LEFT,
            SECOND,
            0.0,
        ));

        // From east's perspective facing west: north is to my RIGHT
        // So east (LEFT) yields to north (DOWN)
        assert!(!resolver.has_priority(
            TurnType::Straight,
            LEFT,
            FIRST,
            0.0,
            TurnType::Straight,
            DOWN,
            SECOND,
            0.0,
        ));
    }

    #[test]
    fn test_yield_to_right_overrides_turn_type() {
        let resolver = YieldResolver::RightOfWay;

        // DOWN = from north, LEFT = from east
        // North has priority over east (east is to north's left)
        // Even though the east car is turning, north still has priority
        assert!(resolver.has_priority(
            TurnType::Straight,
            DOWN,
            FIRST,
            0.0,
            TurnType::Right(-0.7),
            LEFT,
            SECOND,
            0.0,
        ));
    }

    #[test]
    fn test_opposing_directions_right_turn_beats_straight() {
        let resolver = YieldResolver::RightOfWay;

        // Opposing traffic (north vs south): direction_cross ≈ 0
        // Right turn has shorter path than straight
        // Car from north (heading DOWN) turning right vs car from south (heading UP) going straight
        assert!(resolver.has_priority(
            TurnType::Right(-0.7),
            DOWN,
            FIRST,
            0.0,
            TurnType::Straight,
            UP,
            SECOND,
            0.0,
        ));

        // Reverse: straight loses to right turn
        assert!(!resolver.has_priority(
            TurnType::Straight,
            UP,
            FIRST,
            0.0,
            TurnType::Right(-0.7),
            DOWN,
            SECOND,
            0.0,
        ));
    }

    #[test]
    fn test_opposing_directions_right_turn_beats_left_turn() {
        let resolver = YieldResolver::RightOfWay;

        // Both from opposing directions, one turning right, one turning left
        // Right turn (shorter path) wins
        assert!(resolver.has_priority(
            TurnType::Right(-0.7),
            DOWN,
            FIRST,
            0.0,
            TurnType::Left(0.7),
            UP,
            SECOND,
            0.0,
        ));

        assert!(!resolver.has_priority(
            TurnType::Left(0.7),
            UP,
            FIRST,
            0.0,
            TurnType::Right(-0.7),
            DOWN,
            SECOND,
            0.0,
        ));
    }

    #[test]
    fn test_arrival_order_tiebreaker() {
        let resolver = YieldResolver::RightOfWay;

        // Same everything - earlier arrival (lower number) wins
        assert!(resolver.has_priority(
            TurnType::Straight,
            DOWN,
            FIRST,
            0.0,
            TurnType::Straight,
            DOWN,
            SECOND,
            0.0,
        ));

        assert!(!resolver.has_priority(
            TurnType::Straight,
            DOWN,
            SECOND,
            0.0,
            TurnType::Straight,
            DOWN,
            FIRST,
            0.0,
        ));
    }

    #[test]
    fn test_deadlock_breaks_with_arrival_order() {
        let resolver = YieldResolver::RightOfWay;

        // When both vehicles have been waiting > 0.5s, pure FIFO (arrival order) wins
        // regardless of direction or turn type
        // FIRST arrived before SECOND, so FIRST always wins
        assert!(resolver.has_priority(
            TurnType::Straight,
            LEFT,
            FIRST,
            1.0, // east, arrived first, waiting 1s
            TurnType::Straight,
            DOWN,
            SECOND,
            1.0, // north, arrived second, waiting 1s
        ));

        // Even if directions would normally give priority to the other vehicle,
        // arrival order wins in deadlock situation
        assert!(resolver.has_priority(
            TurnType::Straight,
            DOWN,
            FIRST,
            1.0, // north, arrived first
            TurnType::Straight,
            LEFT,
            SECOND,
            1.0, // east, arrived second
        ));

        // SECOND checking against FIRST - SECOND always loses in deadlock
        assert!(!resolver.has_priority(
            TurnType::Straight,
            LEFT,
            SECOND,
            1.0,
            TurnType::Straight,
            DOWN,
            FIRST,
            1.0,
        ));
    }

    #[test]
    fn test_queue_priority_over_non_queue() {
        let resolver = YieldResolver::RightOfWay;
        const NOT_IN_QUEUE: u32 = u32::MAX;

        // Vehicle in queue (FIRST) has priority over vehicle not in queue
        // even if the non-queue vehicle is to our right (normally we'd yield)
        assert!(resolver.has_priority(
            TurnType::Straight,
            LEFT,
            FIRST,
            1.0, // in queue, waiting
            TurnType::Straight,
            DOWN,
            NOT_IN_QUEUE,
            0.0, // not in queue yet
        ));

        // Vehicle not in queue yields to vehicle in queue
        // even if the queue vehicle is to our left (normally we'd have priority)
        assert!(!resolver.has_priority(
            TurnType::Straight,
            DOWN,
            NOT_IN_QUEUE,
            0.0, // not in queue yet
            TurnType::Straight,
            LEFT,
            FIRST,
            1.0, // in queue, waiting
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
            TurnType::Straight,
            from_north,
            FIRST,
            0.0,
            TurnType::Straight,
            from_east,
            SECOND,
            0.0,
        ));

        // East vs North: If I'm at east facing west, north is to my RIGHT
        // So east yields to north
        assert!(!resolver.has_priority(
            TurnType::Straight,
            from_east,
            FIRST,
            0.0,
            TurnType::Straight,
            from_north,
            SECOND,
            0.0,
        ));

        // East vs South: If I'm at east facing west, south is to my LEFT
        // So east has priority over south
        assert!(resolver.has_priority(
            TurnType::Straight,
            from_east,
            FIRST,
            0.0,
            TurnType::Straight,
            from_south,
            SECOND,
            0.0,
        ));

        // South vs East: If I'm at south facing north, east is to my RIGHT
        // So south yields to east
        assert!(!resolver.has_priority(
            TurnType::Straight,
            from_south,
            FIRST,
            0.0,
            TurnType::Straight,
            from_east,
            SECOND,
            0.0,
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
