use bevy::prelude::*;
use simulation::{
    driver::{PlayerControlled, Vehicle},
    Road, SegmentGeometry, SimulationPlugin,
};
use wasm_bindgen::prelude::*;

/// Road width in meters (single lane)
const LANE_WIDTH: f32 = 3.5;

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
        .add_systems(Update, (draw_edge_lines, draw_vehicles, player_input))
        .run();
}

fn setup(mut commands: Commands) {
    // Camera - orthographic 3D looking down at XY plane
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scale: 0.05,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0.0, 0.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub fn test_intersection(mut commands: Commands) {
    let mut road = Road::default();

    // Create nodes
    let north = road.add_edge_node(Vec3::new(10.0, 100.0, 0.0));
    let south = road.add_edge_node(Vec3::new(10.0, -100.0, 0.0));
    let east = road.add_edge_node(Vec3::new(100.0, -10.0, 0.0));
    let west = road.add_edge_node(Vec3::new(-100.0, 10.0, 0.0));
    let center = road.add_node(Vec3::ZERO);

    // Create segments (incoming/outgoing wired automatically)
    road.add_bidirectional(north, center, 5.0);
    road.add_bidirectional(south, center, 5.0);
    road.add_bidirectional(east, center, 5.0);
    road.add_bidirectional(west, center, 5.0);
    // road.add_bidirectional(south_east, center, 5.0);

    // Generate intersection edge nodes and turn segments
    road.finalize();

    commands.insert_resource(road);

    // Spawn player-controlled vehicle
    // commands.spawn((Vehicle::new(seg_north_east), PlayerControlled));
}

/// Spawn road surface meshes for all segments
fn spawn_road_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    road: Res<Road>,
) {
    let road_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22), // Dark gray asphalt
        unlit: true,
        double_sided: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Opaque,
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

        // Add triangles (two per quad) - counter-clockwise winding for front face
        if i < steps {
            let base = (i * 2) as u32;
            // First triangle
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 1);
            // Second triangle
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base + 3);
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

fn draw_vehicles(
    mut gizmos: Gizmos,
    vehicles: Query<(&Vehicle, Option<&PlayerControlled>)>,
    road: Res<Road>,
) {
    for (vehicle, is_player) in &vehicles {
        let segment = road.segments.get(&vehicle.segment);
        let from = road.nodes.get(&segment.from);
        let to = road.nodes.get(&segment.to);

        // Lane offset is baked into node positions by finalize()
        let position = segment
            .geometry
            .position_at(from.position, to.position, vehicle.progress);

        // Calculate heading by sampling two nearby points
        let epsilon = 0.01;
        let t0 = (vehicle.progress - epsilon).max(0.0);
        let t1 = (vehicle.progress + epsilon).min(1.0);
        let p0 = segment.geometry.position_at(from.position, to.position, t0);
        let p1 = segment.geometry.position_at(from.position, to.position, t1);
        let direction = (p1 - p0).normalize_or_zero();

        // Calculate rotation from direction (heading angle around Z axis)
        let angle = direction.y.atan2(direction.x);
        let rotation = Quat::from_rotation_z(angle);

        let color = if is_player.is_some() {
            Color::linear_rgb(0.2, 0.5, 1.0) // Blue for player
        } else {
            Color::linear_rgb(1.0, 0.0, 0.0) // Red for AI
        };

        // Draw car as oriented rectangle using vehicle's dimensions
        gizmos.rect(
            Isometry3d::new(position, rotation),
            Vec2::new(vehicle.length, vehicle.width),
            color,
        );
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
