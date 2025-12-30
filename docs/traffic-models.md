# Traffic Simulation Models

A comprehensive overview of models for traffic AI, with explanations, pros, cons, and implementation notes.

---

## 1. Car-Following Models

These determine how a vehicle follows the one ahead. You accelerate/brake based on gap and relative speed.

### 1.1 IDM (Intelligent Driver Model) ✓ Implemented

The most popular microscopic car-following model. Balances desired speed with safe following.

**Formula:**
```
a = a_max * [1 - (v/v₀)^4 - (s*/s)^2]

s* = s₀ + v*T + (v*Δv) / (2*√(a*b))
```

**Parameters:**
- `v₀` — Desired velocity
- `s₀` — Minimum gap (jam distance)
- `T` — Safe time headway
- `a` — Maximum acceleration
- `b` — Comfortable deceleration

**Pros:**
- Smooth, realistic acceleration curves
- Well-documented, widely used
- Easy to tune with intuitive parameters
- Handles stop-and-go traffic well

**Cons:**
- Can produce negative gaps in edge cases
- No explicit collision avoidance guarantee
- Doesn't handle multi-lane by itself

**Best for:** General-purpose car following, highway traffic

---

### 1.2 Gipps Model

Safe-distance model that guarantees collision-free driving by construction.

**Core Idea:**
Calculate the maximum safe speed that allows stopping before hitting the leader, even if they brake maximally.

**Formula:**
```
v_safe = -b*τ + √[(b*τ)² + v_leader² + 2*b*(gap - s₀)]
v_new = min(v_desired, v_safe, v_current + a*Δt)
```

**Parameters:**
- `τ` — Reaction time
- `b` — Deceleration capability
- `a` — Acceleration capability
- `s₀` — Minimum gap

**Pros:**
- Collision-free by mathematical construction
- Very stable, no oscillations
- Simple to understand

**Cons:**
- Can be overly conservative
- Less smooth than IDM
- Abrupt speed changes possible

**Best for:** Safety-critical simulations, conservative AI drivers

---

### 1.3 Krauss Model

Simplified version of Gipps, used in SUMO traffic simulator.

**Formula:**
```
v_safe = v_leader + (gap - v_leader*τ) / (τ + v_leader/(2*b))
v_desired = min(v_max, v_current + a*Δt)
v_new = max(0, min(v_desired, v_safe) - ε)
```

Where `ε` is a small random imperfection.

**Pros:**
- Very fast to compute
- Good for large-scale simulations
- Built-in randomness for realism

**Cons:**
- Less accurate than Gipps or IDM
- Can produce jerky motion

**Best for:** Large-scale simulations where performance matters

---

### 1.4 Wiedemann Model

Psycho-physical model based on perception thresholds. Drivers react only when they perceive a difference.

**Core Idea:**
Drivers have perception thresholds for:
- Approaching (∆v becoming negative)
- Distance (gap too small)
- Speed difference (closing too fast)

Behavior changes when thresholds are crossed.

**States:**
1. Free driving — No car ahead perceived
2. Approaching — Closing in on leader
3. Following — Matching leader's speed
4. Braking — Emergency deceleration

**Pros:**
- Very realistic human behavior
- Models inattention and reaction delays
- Produces natural speed oscillations

**Cons:**
- Complex, many parameters
- Hard to calibrate
- Computationally expensive

**Best for:** High-fidelity simulations, research

---

### 1.5 Optimal Velocity Model (OVM)

Drivers adjust speed toward an "optimal velocity" based on gap.

**Formula:**
```
a = α * (V_optimal(gap) - v)

V_optimal(gap) = v_max * [tanh(gap - s₀) + tanh(s₀)]
```

**Pros:**
- Simple, elegant math
- Easy to analyze theoretically
- Shows emergent traffic jams

**Cons:**
- Can produce unrealistic collisions
- Overly simplistic for practical use
- No explicit reaction to leader's speed

**Best for:** Traffic flow research, demonstrating jam formation

---

### 1.6 Full Velocity Difference Model (FVDM)

Extension of OVM that adds sensitivity to relative velocity.

**Formula:**
```
a = α * (V_optimal(gap) - v) + β * Δv
```

Where `Δv = v_leader - v` (negative when approaching).

**Pros:**
- Fixes OVM's collision problem
- Still relatively simple
- Better stability

**Cons:**
- Still less realistic than IDM
- Needs careful parameter tuning

**Best for:** Simple simulations needing relative velocity sensitivity

---

## 2. Lane-Changing Models

These decide when and how to change lanes.

### 2.1 MOBIL ✓ Documented

Minimizing Overall Braking Induced by Lane changes. See `mobil-model.md` for details.

**Pros:**
- Pairs perfectly with IDM
- Politeness parameter maps to aggression
- Considers impact on others

**Cons:**
- Requires calculating multiple hypothetical IDM scenarios
- Doesn't model the physical lane-change maneuver

---

### 2.2 LMRS (Lane-change Model with Relaxation and Synchronization)

More sophisticated model that adds:
- **Desire:** How much the driver wants to change lanes
- **Relaxation:** Accepting smaller gaps when necessary
- **Synchronization:** Adjusting speed to create gaps

**States:**
1. Free lane change — Optional, for speed gain
2. Mandatory lane change — Must exit, approaching turn
3. Cooperative — Helping others change lanes

**Pros:**
- Handles mandatory lane changes (exits, turns)
- More realistic gap acceptance under pressure
- Models cooperative behavior

**Cons:**
- More complex than MOBIL
- More parameters to tune
- Requires route/destination awareness

**Best for:** Highway with exits, realistic mandatory merging

---

### 2.3 Gipps Lane-Change Model

Simple gap-based approach: change lanes if gaps are safe.

**Checks:**
1. Is there a safe gap ahead in target lane?
2. Is there a safe gap behind in target lane?
3. Is it beneficial (faster)?

**Pros:**
- Very simple to implement
- Fast computation
- Guaranteed safe

**Cons:**
- No consideration of others
- Can be too conservative or too aggressive
- Doesn't model courtesy

**Best for:** Simple games, baseline behavior

---

## 3. Gap Acceptance Models

Used at intersections, merges, and yields. Decide whether to accept a gap in traffic.

### 3.1 Critical Gap Theory

A driver accepts a gap if it exceeds their critical gap threshold.

**Formula:**
```
accept = gap > t_critical

t_critical = t_base + ε  (where ε is driver-specific variation)
```

**Typical values:**
- Left turn across traffic: 5-7 seconds
- Right turn into traffic: 4-6 seconds
- Merge onto highway: 3-5 seconds

**Pros:**
- Simple and intuitive
- Easy to calibrate from real data
- Deterministic

**Cons:**
- Binary decision (accept/reject)
- Doesn't model increasing urgency
- No learning from rejected gaps

---

### 3.2 Logit Gap Acceptance

Probabilistic model: probability of acceptance increases with gap size.

**Formula:**
```
P(accept) = 1 / (1 + e^(-β * (gap - t_critical)))
```

**Pros:**
- Smooth probability curve
- Models uncertainty
- More realistic decision-making

**Cons:**
- Non-deterministic (randomness)
- Slightly more complex
- Needs probability threshold for decision

**Best for:** Realistic intersection behavior

---

### 3.3 Urgency-Based Acceptance

Critical gap decreases the longer you wait.

**Formula:**
```
t_critical(wait_time) = t_base * e^(-λ * wait_time)
```

**Pros:**
- Models impatient drivers
- Prevents infinite waiting
- Very realistic

**Cons:**
- Can lead to unsafe gap acceptance
- Needs wait time tracking

**Best for:** Games where player causes traffic jams

---

## 4. Route Choice Models

How vehicles choose their path through the network.

### 4.1 Shortest Path (Dijkstra / A*)

Find the path with minimum cost (distance, time, etc.).

**Pros:**
- Optimal solution guaranteed
- Well-understood algorithms
- Fast with proper data structures

**Cons:**
- All vehicles take same "best" route
- Creates unrealistic congestion
- Doesn't adapt to current traffic

**Best for:** Basic navigation, initial route assignment

---

### 4.2 Logit Route Choice

Probabilistic route selection based on route costs.

**Formula:**
```
P(route_i) = e^(-θ * cost_i) / Σ e^(-θ * cost_j)
```

Where `θ` controls randomness (high = more deterministic).

**Pros:**
- Natural distribution across routes
- Avoids everyone taking same path
- Models bounded rationality

**Cons:**
- May choose clearly inferior routes
- Needs multiple route options pre-computed

**Best for:** Realistic traffic distribution

---

### 4.3 Dynamic Traffic Assignment

Re-route based on current congestion.

**Approach:**
1. Compute initial routes
2. Simulate traffic
3. Update travel times based on congestion
4. Re-route vehicles
5. Repeat until equilibrium

**Pros:**
- Responds to congestion
- More realistic flow
- Emergent route patterns

**Cons:**
- Computationally expensive
- Complex to implement
- May oscillate

**Best for:** Large-scale network simulation

---

## 5. Intersection Control Models

### 5.1 Fixed-Time Signals

Phases cycle on fixed timers regardless of traffic.

**Parameters:**
- Cycle length (e.g., 90 seconds)
- Phase splits (e.g., 50% north-south, 50% east-west)
- Yellow time, all-red clearance

**Pros:**
- Simple to implement
- Predictable
- Coordinates well in networks

**Cons:**
- Inefficient for varying demand
- Long waits when no cross-traffic

**Best for:** Initial implementation, baseline

---

### 5.2 Webster's Formula

Optimal cycle length based on flow rates.

**Formula:**
```
C_optimal = (1.5 * L + 5) / (1 - Y)

L = total lost time per cycle
Y = sum of critical flow ratios
```

**Pros:**
- Minimizes average delay
- Based on traffic demand
- Closed-form solution

**Cons:**
- Assumes steady-state traffic
- Doesn't adapt in real-time

**Best for:** Setting good initial signal timings

---

### 5.3 Actuated Signals

Respond to detected vehicles. Extend green while cars are arriving.

**Logic:**
```
if vehicle_detected and green_time < max_green:
    extend_green()
elif gap_too_long or green_time >= max_green:
    switch_phase()
```

**Pros:**
- Adapts to actual traffic
- Reduces unnecessary waiting
- Efficient for variable demand

**Cons:**
- More complex to implement
- Needs vehicle detection
- Can starve low-volume approaches

**Best for:** Realistic signal behavior, player interaction

---

### 5.4 Roundabout Yield

Gap acceptance for circular intersections.

**Rules:**
1. Entering vehicles yield to circulating traffic
2. Apply gap acceptance model
3. Once in circle, priority to continue

**Pros:**
- Continuous flow (no stopping when empty)
- Self-regulating
- Safer than signals

**Cons:**
- Can lock up under heavy load
- Needs careful geometry
- Gap acceptance tuning required

**Best for:** Interesting alternative to signals

---

## 6. Advanced Models

### 6.1 Nagel-Schreckenberg (Cellular Automaton)

Grid-based model. Road divided into cells, cars occupy cells.

**Rules (each timestep):**
1. Acceleration: `v = min(v + 1, v_max)`
2. Slowing: `v = min(v, gap)`
3. Randomization: `v = max(v - 1, 0)` with probability `p`
4. Movement: advance `v` cells

**Pros:**
- Extremely fast (integer math)
- Emergent traffic jams
- Scales to huge networks

**Cons:**
- Discrete, less smooth
- Limited realism
- Harder to calibrate

**Best for:** Large-scale simulations, games with many cars

---

### 6.2 Anticipation (Multi-Leader)

Look at multiple vehicles ahead, not just immediate leader.

**Modified IDM:**
```
a = IDM(leader_1) + γ * IDM(leader_2) + γ² * IDM(leader_3)
```

Where `γ < 1` discounts further leaders.

**Pros:**
- Smooths stop-and-go waves
- More natural braking
- Mimics experienced drivers

**Cons:**
- More computation per vehicle
- Needs visibility/detection range
- Parameter tuning

**Best for:** Reducing unrealistic shockwaves

---

### 6.3 Connected/Cooperative Vehicles

Vehicles share information: position, speed, intent.

**Enables:**
- Platooning (cars follow at very close gaps)
- Cooperative merging
- Collision warnings
- Green wave optimization

**Pros:**
- Dramatically improves throughput
- Safer
- Future-proof (real tech)

**Cons:**
- Requires communication system
- Mixed traffic is complex
- Game design implications

**Best for:** Futuristic game modes, "perfect AI" scenarios

---

## Recommendations for This Game

### Must Have
1. **IDM** ✓ — Core car-following (done!)
2. **Gap Acceptance** — For intersections
3. **Fixed-Time Signals** — Player controls phase timing

### Nice to Have
4. **MOBIL** — If you add multiple lanes
5. **Anticipation** — Reduces traffic wave artifacts
6. **Actuated Signals** — More interesting gameplay

### Future Expansion
7. **Route Choice** — Vehicles with destinations
8. **Urgency-Based Gap** — Interesting failure modes
9. **Connected Vehicles** — Special "autopilot" mode

---

## References

- [Traffic Flow Dynamics (Treiber & Kesting)](https://www.springer.com/gp/book/9783642324598) — The bible of traffic modeling
- [SUMO Documentation](https://sumo.dlr.de/docs/) — Open source simulator with many models
- [traffic-simulation.de](https://traffic-simulation.de/) — Interactive demos of IDM, MOBIL, etc.
