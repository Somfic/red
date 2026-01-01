use bevy::{
    light::{DirectionalLightShadowMap, PointLightShadowMap},
    prelude::*,
    window::PrimaryWindow,
};
use simulation::{
    driver::{Blinker, PlayerControlled, Vehicle, YieldResolver},
    Id, Road, Segment, SegmentGeometry, SimulationPlugin,
};
use wasm_bindgen::prelude::*;

/// Resource tracking which vehicle is currently selected for debug inspection
#[derive(Resource, Default)]
struct SelectedVehicle(Option<Entity>);

/// Resource tracking which segment is currently selected for debug inspection
#[derive(Resource, Default)]
struct SelectedSegment(Option<Id<Segment>>);

/// Road width in meters (single lane)
const LANE_WIDTH: f32 = 3.5;
/// Vehicle height in meters
const CAR_HEIGHT: f32 = 1.2;

/// Marker component for vehicles that have render meshes attached
#[derive(Component)]
struct VehicleRender;

/// Resource holding shared vehicle mesh and materials
#[derive(Resource)]
struct VehicleAssets {
    mesh: Handle<Mesh>,
    ai_material: Handle<StandardMaterial>,
    player_material: Handle<StandardMaterial>,
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                canvas: Some("#game-canvas".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SimulationPlugin)
        .init_resource::<SelectedVehicle>()
        .init_resource::<SelectedSegment>()
        .add_systems(Startup, (setup, test_intersection))
        .add_systems(Startup, spawn_road_meshes.after(test_intersection))
        .add_systems(
            Update,
            (
                draw_edge_lines,
                spawn_vehicle_meshes,
                update_vehicle_transforms,
                draw_vehicle_lights,
                player_input,
                handle_selection,
                draw_selected_vehicle_debug,
                draw_selected_segment_debug,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Isometric camera setup
    // Classic isometric: 45° rotation around vertical, ~30° elevation angle
    let distance = 120.0;
    let look_at = Vec3::new(80.0, 0.0, 0.0); // Center between roundabout and intersection

    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scale: 0.18, // Zoomed out to see both intersections
            ..OrthographicProjection::default_3d()
        }),
        // Position camera at isometric angle: (d, -d, d*0.7) looking at center
        Transform::from_xyz(look_at.x + distance, look_at.y - distance, distance * 0.7)
            .looking_at(look_at, Vec3::Z),
    ));

    // Simulate directional light with distant point light
    // Far away = nearly parallel rays like sunlight
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            shadow_depth_bias: 0.01,
            illuminance: 20000.0,
            ..default()
        },
        // Far away, low angle for long shadows
        Transform::from_xyz(-500.0, 500.0, 200.0),
    ));

    // Low ambient so shadows are visible
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 80.0,
        ..default()
    });

    // Higher resolution shadow map
    commands.insert_resource(DirectionalLightShadowMap {
        size: 2048, // try 1024 or 2048 on web
    });

    // Create shared vehicle assets
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let ai_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1), // Red for AI
        ..default()
    });

    let player_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.5, 1.0), // Blue for player
        ..default()
    });

    commands.insert_resource(VehicleAssets {
        mesh,
        ai_material,
        player_material,
    });
}

pub fn test_intersection(mut commands: Commands) {
    let mut road = Road::default();

    let spacing = 80.0;

    // Roundabout in the center
    let roundabout =
        road.add_intersection_node(Vec3::new(0.0, 0.0, 0.0), YieldResolver::Roundabout);

    // Regular intersection to the east
    let intersection =
        road.add_intersection_node(Vec3::new(spacing * 2.0, 0.0, 0.0), YieldResolver::RightOfWay);

    // Edge nodes around the roundabout
    let edge_n = road.add_edge_node(Vec3::new(0.0, spacing, 0.0));
    let edge_s = road.add_edge_node(Vec3::new(0.0, -spacing, 0.0));
    let edge_w = road.add_edge_node(Vec3::new(-spacing, 0.0, 0.0));

    // Edge nodes around the regular intersection
    let int_n = road.add_edge_node(Vec3::new(spacing * 2.0, spacing, 0.0));
    let int_s = road.add_edge_node(Vec3::new(spacing * 2.0, -spacing, 0.0));
    let int_e = road.add_edge_node(Vec3::new(spacing * 3.0, 0.0, 0.0));

    // Connect roundabout to edge nodes
    road.add_bidirectional(edge_n, roundabout, 13.9);
    road.add_bidirectional(edge_s, roundabout, 13.9);
    road.add_bidirectional(edge_w, roundabout, 13.9);

    // Connect roundabout to intersection
    road.add_bidirectional(roundabout, intersection, 13.9);

    // Connect intersection to its edge nodes
    road.add_bidirectional(int_n, intersection, 13.9);
    road.add_bidirectional(int_s, intersection, 13.9);
    road.add_bidirectional(int_e, intersection, 13.9);

    // Generate intersection edge nodes and turn segments
    road.finalize();

    commands.insert_resource(road);
}

/// Spawn road surface meshes for all segments
fn spawn_road_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    road: Res<Road>,
) {
    let road_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.32), // Dark gray asphalt
        ..default()
    });

    for segment in road.segments.iter() {
        let from = road.nodes.get(&segment.from);
        let to = road.nodes.get(&segment.to);

        let mesh = build_segment_mesh(&segment.geometry, from.position, to.position, LANE_WIDTH);

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(road_material.clone()),
            Transform::from_xyz(0.0, 0.0, -0.01),
            Visibility::Visible,
        ));
    }
}

/// Build a quad strip mesh along a segment path
fn build_segment_mesh(geometry: &SegmentGeometry, from: Vec3, to: Vec3, width: f32) -> Mesh {
    let steps = match geometry {
        SegmentGeometry::Straight => 1,
        SegmentGeometry::Curved { .. } => 16,
    };

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((steps + 1) * 2);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((steps + 1) * 2);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((steps + 1) * 2);
    let mut indices: Vec<u32> = Vec::with_capacity(steps * 6);

    let half_width = width / 2.0;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let center = geometry.position_at(from, to, t);

        // Calculate tangent direction
        let epsilon = 0.001;
        let t0 = (t - epsilon).max(0.0);
        let t1 = (t + epsilon).min(1.0);
        let p0 = geometry.position_at(from, to, t0);
        let p1 = geometry.position_at(from, to, t1);
        let tangent = (p1 - p0).normalize_or_zero();

        // Perpendicular (90° rotation in XY plane)
        let perp = Vec3::new(-tangent.y, tangent.x, 0.0);

        // Left and right edge positions
        let left = center + perp * half_width;
        let right = center - perp * half_width;

        positions.push([left.x, left.y, left.z]);
        positions.push([right.x, right.y, right.z]);
        normals.push([0.0, 0.0, 1.0]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([0.0, t]);
        uvs.push([1.0, t]);

        // Add triangles (two per quad) - counter-clockwise winding for +Z normal
        if i < steps {
            let base = (i * 2) as u32;
            // First triangle (flipped winding)
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            // Second triangle (flipped winding)
            indices.push(base + 1);
            indices.push(base + 3);
            indices.push(base + 2);
        }
    }

    Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

/// Draw white edge lines on road boundaries
/// - Solid lines on approach roads (outside intersections)
/// - Solid perimeter around intersections
fn draw_edge_lines(mut gizmos: Gizmos, road: Res<Road>) {
    let half_width = LANE_WIDTH / 2.0;
    let edge_color = Color::linear_rgb(0.9, 0.9, 0.9); // White

    // Collect all intersection edge nodes for checking
    let intersection_nodes: Vec<_> = road
        .intersections
        .iter()
        .flat_map(|i| i.edge_nodes.iter().copied())
        .collect();

    // Draw all segment edge lines
    for (seg_id, segment) in road.segments.iter_with_ids() {
        let from = road.nodes.get(&segment.from);
        let to = road.nodes.get(&segment.to);

        // Check if this segment is inside an intersection
        // (both endpoints are edge nodes of the same intersection)
        let intersection = road
            .intersections
            .iter()
            .find(|i| i.edge_nodes.contains(&segment.from) && i.edge_nodes.contains(&segment.to));

        // For intersection segments, check if it's an outer corner segment
        let is_outer_corner = if let Some(inter) = intersection {
            // Sort edge nodes by angle around center
            let center = inter.position;
            let mut sorted_nodes: Vec<_> = inter.edge_nodes.iter().copied().collect();
            sorted_nodes.sort_by(|a, b| {
                let pos_a = road.nodes.get(a).position;
                let pos_b = road.nodes.get(b).position;
                let angle_a = (pos_a.y - center.y).atan2(pos_a.x - center.x);
                let angle_b = (pos_b.y - center.y).atan2(pos_b.x - center.x);
                angle_a.partial_cmp(&angle_b).unwrap()
            });

            // Check if from and to are adjacent in the sorted order
            let from_idx = sorted_nodes.iter().position(|&n| n == segment.from);
            let to_idx = sorted_nodes.iter().position(|&n| n == segment.to);

            if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                let len = sorted_nodes.len();
                let diff = (fi as i32 - ti as i32).abs();
                // Adjacent if indices differ by 1, or wrap around
                diff == 1 || diff == (len as i32 - 1)
            } else {
                false
            }
        } else {
            false
        };

        // Skip inner crossing segments entirely
        if intersection.is_some() && !is_outer_corner {
            continue;
        }

        let steps = match segment.geometry {
            SegmentGeometry::Straight => 1,
            SegmentGeometry::Curved { .. } => 16,
        };

        for i in 0..steps {
            let t0 = i as f32 / steps as f32;
            let t1 = (i + 1) as f32 / steps as f32;

            let c0 = segment.geometry.position_at(from.position, to.position, t0);
            let c1 = segment.geometry.position_at(from.position, to.position, t1);

            let tangent = (c1 - c0).normalize_or_zero();
            let perp = Vec3::new(-tangent.y, tangent.x, 0.0);

            let left0 = c0 + perp * half_width + Vec3::Z * 0.01;
            let left1 = c1 + perp * half_width + Vec3::Z * 0.01;
            let right0 = c0 - perp * half_width + Vec3::Z * 0.01;
            let right1 = c1 - perp * half_width + Vec3::Z * 0.01;

            if intersection.is_some() {
                // For outer corner segments, only draw the outer edge (further from center)
                let inter = intersection.unwrap();
                let mid = (c0 + c1) / 2.0;
                let left_mid = mid + perp * half_width;
                let right_mid = mid - perp * half_width;

                let left_dist = left_mid.distance(inter.position);
                let right_dist = right_mid.distance(inter.position);

                if left_dist > right_dist {
                    gizmos.line(left0, left1, edge_color);
                } else {
                    gizmos.line(right0, right1, edge_color);
                }

                // Draw dashed guide line along inner edge (skip every other segment)
                if i % 2 == 0 {
                    // Inner edge is the one closer to intersection center
                    if left_dist > right_dist {
                        // Left is outer, so right is inner
                        gizmos.line(right0, right1, edge_color);
                    } else {
                        // Right is outer, so left is inner
                        gizmos.line(left0, left1, edge_color);
                    }
                }
            } else {
                // For regular segments, draw both edges
                gizmos.line(left0, left1, edge_color);
                gizmos.line(right0, right1, edge_color);
            }
        }
    }
}

/// Spawn mesh components for vehicles that don't have them yet
fn spawn_vehicle_meshes(
    mut commands: Commands,
    vehicles: Query<(Entity, Option<&PlayerControlled>), (With<Vehicle>, Without<VehicleRender>)>,
    assets: Res<VehicleAssets>,
) {
    for (entity, is_player) in &vehicles {
        let material = if is_player.is_some() {
            assets.player_material.clone()
        } else {
            assets.ai_material.clone()
        };

        commands.entity(entity).insert((
            VehicleRender,
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Visible,
        ));
    }
}

/// Update vehicle mesh transforms based on simulation state
fn update_vehicle_transforms(
    mut vehicles: Query<(&Vehicle, &mut Transform), With<VehicleRender>>,
    road: Res<Road>,
) {
    for (vehicle, mut transform) in &mut vehicles {
        let segment = road.segments.get(&vehicle.segment);
        let from = road.nodes.get(&segment.from);
        let to = road.nodes.get(&segment.to);

        let position = segment
            .geometry
            .position_at(from.position, to.position, vehicle.progress);

        // Calculate heading
        let epsilon = 0.01;
        let t0 = (vehicle.progress - epsilon).max(0.0);
        let t1 = (vehicle.progress + epsilon).min(1.0);
        let p0 = segment.geometry.position_at(from.position, to.position, t0);
        let p1 = segment.geometry.position_at(from.position, to.position, t1);
        let direction = (p1 - p0).normalize_or_zero();
        let angle = direction.y.atan2(direction.x);

        // Position at center of car (raised by half height)
        let car_center = position + Vec3::Z * (CAR_HEIGHT / 2.0);

        *transform = Transform::from_translation(car_center)
            .with_rotation(Quat::from_rotation_z(angle))
            .with_scale(Vec3::new(vehicle.length, vehicle.width, CAR_HEIGHT));
    }
}

/// Draw vehicle lights (blinkers, brake lights) using gizmos
fn draw_vehicle_lights(
    mut gizmos: Gizmos,
    vehicles: Query<(&Vehicle, &Transform)>,
    time: Res<Time>,
) {
    let blink_on = (time.elapsed_secs() * 2.0) as i32 % 2 == 0;
    let blinker_color = Color::linear_rgb(1.0, 0.7, 0.0);

    for (vehicle, transform) in &vehicles {
        let position = transform.translation - Vec3::Z * (CAR_HEIGHT / 2.0);
        let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
        let perp = Vec3::new(-direction.y, direction.x, 0.0);

        let half_length = vehicle.length / 2.0;
        let half_width = vehicle.width / 2.0;
        let light_size = 0.35;
        let light_height = CAR_HEIGHT * 0.4;

        let front_left =
            position + direction * half_length + perp * half_width + Vec3::Z * light_height;
        let front_right =
            position + direction * half_length - perp * half_width + Vec3::Z * light_height;
        let rear_left =
            position - direction * half_length + perp * half_width + Vec3::Z * light_height;
        let rear_right =
            position - direction * half_length - perp * half_width + Vec3::Z * light_height;

        // Brake lights
        if vehicle.braking {
            let brake_color = Color::linear_rgb(1.0, 0.0, 0.0);
            gizmos.sphere(rear_left, light_size, brake_color);
            gizmos.sphere(rear_right, light_size, brake_color);
        }

        // Blinkers
        if blink_on && vehicle.blinker != Blinker::None {
            match vehicle.blinker {
                Blinker::Left => {
                    gizmos.sphere(front_left, light_size, blinker_color);
                    gizmos.sphere(rear_left, light_size, blinker_color);
                }
                Blinker::Right => {
                    gizmos.sphere(front_right, light_size, blinker_color);
                    gizmos.sphere(rear_right, light_size, blinker_color);
                }
                Blinker::None => {}
            }
        }
    }
}

fn player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player: Query<&mut Vehicle, With<PlayerControlled>>,
    time: Res<Time>,
) {
    let Ok(mut vehicle) = player.single_mut() else {
        return;
    };

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        vehicle.speed += 5.0 * time.delta_secs(); // Accelerate
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        vehicle.speed -= 8.0 * time.delta_secs(); // Brake
    }

    vehicle.speed = vehicle.speed.clamp(0.0, 10.0);
}

/// Handle mouse clicks to select vehicles or segments for debug inspection
fn handle_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    vehicles: Query<(Entity, &Vehicle, &Transform), With<VehicleRender>>,
    all_vehicles: Query<(Entity, &Vehicle)>,
    mut selected_vehicle: ResMut<SelectedVehicle>,
    mut selected_segment: ResMut<SelectedSegment>,
    road: Res<Road>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window.single() else { return };
    let Ok((camera, cam_transform)) = camera.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Convert cursor position to world coordinates
    // For orthographic camera, we can use viewport_to_world and get the XY intersection with Z=0
    let Ok(ray) = camera.viewport_to_world(cam_transform, cursor_pos) else {
        return;
    };

    // Find where ray intersects Z=0 plane
    let t = -ray.origin.z / ray.direction.z;
    let world_pos = ray.origin + ray.direction * t;

    // Find nearest vehicle to click position (using XY distance)
    let mut nearest_vehicle: Option<(Entity, f32)> = None;
    for (entity, _vehicle, transform) in &vehicles {
        let vehicle_pos = transform.translation;
        let dist =
            ((world_pos.x - vehicle_pos.x).powi(2) + (world_pos.y - vehicle_pos.y).powi(2)).sqrt();

        // Check if within selection radius (roughly vehicle size)
        if dist < 5.0 {
            if nearest_vehicle.is_none() || dist < nearest_vehicle.unwrap().1 {
                nearest_vehicle = Some((entity, dist));
            }
        }
    }

    if let Some((entity, _)) = nearest_vehicle {
        selected_vehicle.0 = Some(entity);
        selected_segment.0 = None;

        // Log debug info to console
        if let Ok((_, vehicle)) = all_vehicles.get(entity) {
            log_vehicle_debug(entity, vehicle, &all_vehicles, &road);
        }
        return;
    }

    // No vehicle clicked - check for segment
    let mut nearest_segment: Option<(Id<Segment>, f32)> = None;
    for (seg_id, segment) in road.segments.iter_with_ids() {
        let from = road.nodes.get(&segment.from).position;
        let to = road.nodes.get(&segment.to).position;

        // Sample points along the segment to find distance
        let steps = match segment.geometry {
            SegmentGeometry::Straight => 4,
            SegmentGeometry::Curved { .. } => 16,
        };

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = segment.geometry.position_at(from, to, t);
            let dist = ((world_pos.x - pos.x).powi(2) + (world_pos.y - pos.y).powi(2)).sqrt();

            if dist < LANE_WIDTH {
                if nearest_segment.is_none() || dist < nearest_segment.unwrap().1 {
                    nearest_segment = Some((seg_id, dist));
                }
            }
        }
    }

    if let Some((seg_id, _)) = nearest_segment {
        selected_segment.0 = Some(seg_id);
        selected_vehicle.0 = None;

        // Log segment info
        let segment = road.segments.get(&seg_id);
        let mut output = format!("\n=== Segment Debug ===\nSegment: {:?}\n", seg_id);
        output.push_str(&format!(
            "From: {:?} -> To: {:?}\n",
            segment.from, segment.to
        ));
        output.push_str(&format!("Length: {:.1}m\n", segment.length));

        // Find conflicts
        for intersection in road.intersections.iter() {
            if intersection.incoming.contains(&seg_id) {
                if let Some(conflicts) = intersection.conflicts.get(&seg_id) {
                    output.push_str(&format!("\nConflicting segments: {:?}\n", conflicts));
                }
                break;
            }
        }

        web_sys::console::log_1(&output.into());
    } else {
        selected_vehicle.0 = None;
        selected_segment.0 = None;
    }
}

/// Log vehicle debug info to browser console
fn log_vehicle_debug(
    entity: Entity,
    vehicle: &Vehicle,
    all_vehicles: &Query<(Entity, &Vehicle)>,
    road: &Road,
) {
    let mut output = String::new();

    output.push_str("\n=== Vehicle Debug ===\n");
    output.push_str(&format!("Entity: {:?}\n", entity));
    output.push_str(&format!("Speed: {:.2} m/s\n", vehicle.speed));
    output.push_str(&format!("Progress: {:.2}\n", vehicle.progress));
    output.push_str(&format!("Segment: {:?}", vehicle.segment));

    if let Some(next_seg) = vehicle.route.get(1) {
        output.push_str(&format!(" -> Next: {:?}\n", next_seg));
    } else {
        output.push_str(" -> (no next segment)\n");
    }

    // Log full route with segment details
    output.push_str(&format!("Destination: {:?}\n", vehicle.destination));
    output.push_str(&format!("\nRoute ({} segments):\n", vehicle.route.len()));
    for (i, seg_id) in vehicle.route.iter().enumerate() {
        let seg = road.segments.get(seg_id);
        let from_pos = road.nodes.get(&seg.from).position;
        let to_pos = road.nodes.get(&seg.to).position;
        let marker = if i == 0 { " <-- current" } else { "" };
        output.push_str(&format!(
            "  [{}] {:?}: {:?} ({:.0},{:.0}) -> ({:.0},{:.0}), len={:.1}m{}\n",
            i,
            seg_id,
            seg.turn_type,
            from_pos.x,
            from_pos.y,
            to_pos.x,
            to_pos.y,
            seg.length,
            marker
        ));
    }

    output.push_str("\nGap Acceptance:\n");
    output.push_str(&format!(
        "  arrival_order: {:?}\n",
        vehicle.gap.arrival_order
    ));
    output.push_str(&format!("  waiting_time: {:?}\n", vehicle.gap.waiting_time));
    output.push_str(&format!("  cleared_to_go: {}\n", vehicle.gap.cleared_to_go));
    output.push_str(&format!("  min_gap: {:.2}s\n", vehicle.gap.min_gap));

    // Find conflicts if approaching intersection
    if vehicle.progress > 0.5 {
        if let Some(next_seg) = vehicle.route.get(1) {
            for intersection in road.intersections.iter() {
                if intersection.incoming.contains(next_seg) {
                    if let Some(conflicts) = intersection.conflicts.get(next_seg) {
                        output.push_str("\nConflicts:\n");

                        let my_arrival = vehicle.gap.arrival_order.unwrap_or(u32::MAX);
                        let my_waiting = vehicle.gap.waiting_time.unwrap_or(0.0);

                        for (other_entity, other_vehicle) in all_vehicles.iter() {
                            if other_entity == entity {
                                continue;
                            }

                            // Check if other vehicle is on or approaching a conflicting segment
                            let on_conflict = conflicts.contains(&other_vehicle.segment);
                            let approaching_conflict = other_vehicle
                                .route
                                .get(1)
                                .map(|s| conflicts.contains(s))
                                .unwrap_or(false);

                            if on_conflict || approaching_conflict {
                                let their_arrival =
                                    other_vehicle.gap.arrival_order.unwrap_or(u32::MAX);
                                let their_waiting = other_vehicle.gap.waiting_time.unwrap_or(0.0);

                                let priority_status = if on_conflict {
                                    "IN INTERSECTION - must wait"
                                } else if my_arrival < their_arrival {
                                    "I have priority (arrived first)"
                                } else if my_arrival > their_arrival {
                                    "THEY have priority (arrived first)"
                                } else {
                                    "Same arrival order??"
                                };

                                output.push_str(&format!(
                                    "  - {:?}: arrival={:?}, waiting={:.2}s, {}\n",
                                    other_entity, their_arrival, their_waiting, priority_status
                                ));
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    web_sys::console::log_1(&output.into());
}

/// Draw debug visualization for selected vehicle
fn draw_selected_vehicle_debug(
    mut gizmos: Gizmos,
    selected: Res<SelectedVehicle>,
    vehicles: Query<(Entity, &Vehicle, &Transform)>,
    road: Res<Road>,
) {
    let Some(selected_entity) = selected.0 else {
        return;
    };

    let Ok((_, vehicle, transform)) = vehicles.get(selected_entity) else {
        return;
    };

    let position = transform.translation;

    // Draw yellow selection highlight box
    let half_len = vehicle.length / 2.0 + 0.5;
    let half_wid = vehicle.width / 2.0 + 0.5;
    let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
    let dir = Vec3::new(angle.cos(), angle.sin(), 0.0);
    let perp = Vec3::new(-dir.y, dir.x, 0.0);

    let corners = [
        position + dir * half_len + perp * half_wid,
        position + dir * half_len - perp * half_wid,
        position - dir * half_len - perp * half_wid,
        position - dir * half_len + perp * half_wid,
    ];

    let yellow = Color::linear_rgb(1.0, 1.0, 0.0);
    for i in 0..4 {
        gizmos.line(
            corners[i] + Vec3::Z * 2.0,
            corners[(i + 1) % 4] + Vec3::Z * 2.0,
            yellow,
        );
    }

    // Draw the planned route
    let route_z = Vec3::Z * 1.5;
    for (i, seg_id) in vehicle.route.iter().enumerate() {
        let seg = road.segments.get(seg_id);
        let from_pos = road.nodes.get(&seg.from).position;
        let to_pos = road.nodes.get(&seg.to).position;

        // Color: bright cyan for current, fading to dimmer for future segments
        let brightness = 1.0 - (i as f32 * 0.15).min(0.7);
        let color = Color::linear_rgb(0.0, brightness, brightness);

        // Draw the segment path
        let steps = match seg.geometry {
            SegmentGeometry::Straight => 1,
            SegmentGeometry::Curved { .. } => 12,
        };

        for j in 0..steps {
            let t0 = j as f32 / steps as f32;
            let t1 = (j + 1) as f32 / steps as f32;
            let p0 = seg.geometry.position_at(from_pos, to_pos, t0) + route_z;
            let p1 = seg.geometry.position_at(from_pos, to_pos, t1) + route_z;
            gizmos.line(p0, p1, color);
        }

        // Draw segment index number at midpoint
        let mid = seg.geometry.position_at(from_pos, to_pos, 0.5) + route_z;
        gizmos.circle(mid, 1.0, color);
    }

    // Draw destination marker
    let dest_pos = road.nodes.get(&vehicle.destination).position + Vec3::Z * 2.0;
    let green = Color::linear_rgb(0.0, 1.0, 0.0);
    gizmos.circle(dest_pos, 3.0, green);
    gizmos.circle(dest_pos, 2.0, green);

    // Draw conflict lines if approaching intersection
    if vehicle.progress > 0.5 {
        if let Some(next_seg) = vehicle.route.get(1) {
            for intersection in road.intersections.iter() {
                if intersection.incoming.contains(next_seg) {
                    if let Some(conflicts) = intersection.conflicts.get(next_seg) {
                        let my_arrival = vehicle.gap.arrival_order.unwrap_or(u32::MAX);

                        for (other_entity, other_vehicle, other_transform) in &vehicles {
                            if other_entity == selected_entity {
                                continue;
                            }

                            // Check if other vehicle is on or approaching a conflicting segment
                            let on_conflict = conflicts.contains(&other_vehicle.segment);
                            let approaching_conflict = other_vehicle
                                .route
                                .get(1)
                                .map(|s| conflicts.contains(s))
                                .unwrap_or(false);

                            if on_conflict || approaching_conflict {
                                let their_arrival =
                                    other_vehicle.gap.arrival_order.unwrap_or(u32::MAX);

                                // Red = they have priority (I must yield)
                                // Green = I have priority over them
                                let color = if on_conflict {
                                    Color::linear_rgb(1.0, 0.0, 0.0) // Red - in intersection
                                } else if my_arrival < their_arrival {
                                    Color::linear_rgb(0.0, 1.0, 0.0) // Green - I win
                                } else {
                                    Color::linear_rgb(1.0, 0.0, 0.0) // Red - they win
                                };

                                let other_pos = other_transform.translation + Vec3::Z * 1.5;
                                gizmos.line(position + Vec3::Z * 1.5, other_pos, color);
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
}

/// Draw debug visualization for selected segment
fn draw_selected_segment_debug(
    mut gizmos: Gizmos,
    selected: Res<SelectedSegment>,
    road: Res<Road>,
) {
    let Some(selected_seg_id) = selected.0 else {
        return;
    };

    let segment = road.segments.get(&selected_seg_id);
    let from = road.nodes.get(&segment.from).position;
    let to = road.nodes.get(&segment.to).position;

    // Draw selected segment in yellow
    draw_segment_gizmo(
        &mut gizmos,
        &segment.geometry,
        from,
        to,
        Color::linear_rgb(1.0, 1.0, 0.0),
    );

    // Find intersection containing this segment
    for intersection in road.intersections.iter() {
        if intersection.incoming.contains(&selected_seg_id) {
            let conflicts = intersection.conflicts.get(&selected_seg_id);

            // Draw all other segments in the intersection
            for other_seg_id in &intersection.incoming {
                if *other_seg_id == selected_seg_id {
                    continue;
                }

                let other_seg = road.segments.get(other_seg_id);
                let other_from = road.nodes.get(&other_seg.from).position;
                let other_to = road.nodes.get(&other_seg.to).position;

                // Red if conflicting, green if not
                let is_conflict = conflicts.map(|c| c.contains(other_seg_id)).unwrap_or(false);
                let color = if is_conflict {
                    Color::linear_rgb(1.0, 0.0, 0.0) // Red - conflicts
                } else {
                    Color::linear_rgb(0.0, 1.0, 0.0) // Green - no conflict
                };

                draw_segment_gizmo(
                    &mut gizmos,
                    &other_seg.geometry,
                    other_from,
                    other_to,
                    color,
                );
            }
            break;
        }
    }
}

/// Helper to draw a segment path using gizmos
fn draw_segment_gizmo(
    gizmos: &mut Gizmos,
    geometry: &SegmentGeometry,
    from: Vec3,
    to: Vec3,
    color: Color,
) {
    let steps = match geometry {
        SegmentGeometry::Straight => 1,
        SegmentGeometry::Curved { .. } => 16,
    };

    let z_offset = Vec3::Z * 0.5;
    let half_width = LANE_WIDTH / 2.0;

    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;

        let c0 = geometry.position_at(from, to, t0);
        let c1 = geometry.position_at(from, to, t1);

        let tangent = (c1 - c0).normalize_or_zero();
        let perp = Vec3::new(-tangent.y, tangent.x, 0.0);

        // Draw both edges of the segment
        let left0 = c0 + perp * half_width + z_offset;
        let left1 = c1 + perp * half_width + z_offset;
        let right0 = c0 - perp * half_width + z_offset;
        let right1 = c1 - perp * half_width + z_offset;

        gizmos.line(left0, left1, color);
        gizmos.line(right0, right1, color);

        // Draw center line
        gizmos.line(c0 + z_offset, c1 + z_offset, color);
    }
}
