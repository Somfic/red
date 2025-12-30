# MOBIL (Minimizing Overall Braking Induced by Lane changes)

MOBIL decides **when** a lane change is safe and beneficial. It pairs with IDM which handles the **following** behavior.

## Core Idea

A vehicle considers changing lanes if:
1. **It's safe** — Won't cause the car behind (in target lane) to brake too hard
2. **It's beneficial** — I gain more than others lose

## The Formula

```
Incentive = (a'_self - a_self) + p * [(a'_behind - a_behind) + (a'_old_behind - a_old_behind)]
```

Where:
- `a_self` = my current acceleration (in my lane)
- `a'_self` = my acceleration if I change lanes
- `a_behind` = car behind me in target lane (before change)
- `a'_behind` = car behind me in target lane (after I cut in)
- `a_old_behind` = car behind me in current lane (before)
- `a'_old_behind` = car behind me in current lane (after I leave)
- `p` = **politeness factor** (0 = selfish, 1 = considerate)

## Safety Criterion

Only change if the new follower won't have to brake too hard:
```
a'_behind > -b_safe
```
Where `b_safe` is the safe braking threshold (e.g., -4 m/s²).

## Politeness Factor `p`

This is where **aggression** comes in:
- `p = 0` — Selfish: "I only care about my own gain"
- `p = 0.5` — Balanced: "I'll change if it helps overall"
- `p = 1` — Polite: "I won't inconvenience others much"

Aggressive drivers → low `p` → more lane changes
Calm drivers → high `p` → fewer lane changes

## Visual Example

```
Lane 1:  [A]----[B]--------[C]----
Lane 2:  ----[D]----[E]-----------

Can B move to Lane 2?
- Would B be faster behind D instead of behind A?
- Would E have to slam brakes if B cuts in?
- Does B's gain outweigh E's loss?
```

## Implementation Sketch

```rust
pub struct Mobil {
    pub politeness: f32,      // 0.0 - 1.0
    pub safe_braking: f32,    // e.g., 4.0
    pub threshold: f32,       // minimum incentive to bother changing
}

impl Mobil {
    pub fn should_change_lane(
        &self,
        my_accel_current: f32,
        my_accel_target: f32,
        behind_accel_before: f32,
        behind_accel_after: f32,
        old_behind_accel_before: f32,
        old_behind_accel_after: f32,
    ) -> bool {
        // Safety check
        if behind_accel_after < -self.safe_braking {
            return false;
        }

        // Incentive calculation
        let my_gain = my_accel_target - my_accel_current;
        let others_loss = self.politeness * (
            (behind_accel_after - behind_accel_before) +
            (old_behind_accel_after - old_behind_accel_before)
        );

        my_gain + others_loss > self.threshold
    }
}
```

## Integration with IDM

To evaluate a potential lane change, you run IDM calculations for hypothetical scenarios:

1. Calculate `a_self` using IDM with current lane's leader
2. Calculate `a'_self` using IDM with target lane's leader
3. Calculate `a_behind` for target lane follower with their current leader
4. Calculate `a'_behind` for target lane follower with ME as their new leader
5. Similarly for old lane follower
6. Plug all values into MOBIL formula

## References

- [Original MOBIL Paper](https://traffic-simulation.de/MOBIL.pdf)
- [IDM + MOBIL Interactive Demo](https://traffic-simulation.de/)
