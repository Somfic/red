use bevy::{
    light::{DirectionalLightShadowMap, PointLightShadowMap},
    prelude::*,
};
use simulation::{
    driver::{Blinker, PlayerControlled, Vehicle, YieldResolver},
    Road, SegmentGeometry, SimulationPlugin,
};
use wasm_bindgen::prelude::*;

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

    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scale: 0.09,
            ..OrthographicProjection::default_3d()
        }),
        // Position camera at isometric angle: (d, -d, d*0.7) looking at center
        Transform::from_xyz(distance, -distance, distance * 0.7).looking_at(Vec3::ZERO, Vec3::Z),
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

    // Grid of intersections
    let spacing = 80.0;

    // Create intersection nodes (2x2 grid)
    let int_nw = road.add_intersection_node(
        Vec3::new(-spacing / 2.0, spacing / 2.0, 0.0),
        YieldResolver::RightOfWay,
    );
    let int_ne = road.add_intersection_node(
        Vec3::new(spacing / 2.0, spacing / 2.0, 0.0),
        YieldResolver::RightOfWay,
    );
    let int_sw = road.add_intersection_node(
        Vec3::new(-spacing / 2.0, -spacing / 2.0, 0.0),
        YieldResolver::RightOfWay,
    );
    let int_se = road.add_intersection_node(
        Vec3::new(spacing / 2.0, -spacing / 2.0, 0.0),
        YieldResolver::RightOfWay,
    );

    // Create edge nodes (entry/exit points)
    let edge_n1 = road.add_edge_node(Vec3::new(-spacing / 2.0, spacing * 1.5, 0.0));
    let edge_n2 = road.add_edge_node(Vec3::new(spacing / 2.0, spacing * 1.5, 0.0));
    let edge_s1 = road.add_edge_node(Vec3::new(-spacing / 2.0, -spacing * 1.5, 0.0));
    let edge_s2 = road.add_edge_node(Vec3::new(spacing / 2.0, -spacing * 1.5, 0.0));
    let edge_w1 = road.add_edge_node(Vec3::new(-spacing * 1.5, spacing / 2.0, 0.0));
    let edge_w2 = road.add_edge_node(Vec3::new(-spacing * 1.5, -spacing / 2.0, 0.0));
    let edge_e1 = road.add_edge_node(Vec3::new(spacing * 1.5, spacing / 2.0, 0.0));
    let edge_e2 = road.add_edge_node(Vec3::new(spacing * 1.5, -spacing / 2.0, 0.0));

    // Connect intersections horizontally
    road.add_bidirectional(int_nw, int_ne, 13.9);
    road.add_bidirectional(int_sw, int_se, 13.9);

    // Connect intersections vertically
    road.add_bidirectional(int_nw, int_sw, 13.9);
    road.add_bidirectional(int_ne, int_se, 13.9);

    // Connect to edge nodes (entry/exit roads)
    road.add_bidirectional(edge_n1, int_nw, 13.9);
    road.add_bidirectional(edge_n2, int_ne, 13.9);
    road.add_bidirectional(edge_s1, int_sw, 13.9);
    road.add_bidirectional(edge_s2, int_se, 13.9);
    road.add_bidirectional(edge_w1, int_nw, 13.9);
    road.add_bidirectional(edge_w2, int_sw, 13.9);
    road.add_bidirectional(edge_e1, int_ne, 13.9);
    road.add_bidirectional(edge_e2, int_se, 13.9);

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
