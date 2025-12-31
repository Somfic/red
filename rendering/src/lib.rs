use bevy::prelude::*;
use simulation::{
    driver::{Blinker, PlayerControlled, Vehicle, YieldResolver},
    Road, SegmentGeometry, SimulationPlugin,
};
use wasm_bindgen::prelude::*;

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
        .add_systems(Update, (draw_segments, draw_vehicles, player_input))
        .run();
}

fn setup(mut commands: Commands) {
    // Light
    commands.spawn((PointLight::default(), Transform::from_xyz(0.0, 0.0, 4.0)));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scale: 0.05,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub fn test_intersection(mut commands: Commands) {
    let mut road = Road::default();

    // Create nodes
    let north = road.add_edge_node(Vec3::new(0.0, 100.0, 0.0));
    let south = road.add_edge_node(Vec3::new(30.0, -100.0, 0.0));
    let east = road.add_edge_node(Vec3::new(100.0, -20.0, 0.0));
    let west = road.add_edge_node(Vec3::new(-100.0, 100.0, 0.0));
    let center = road.add_node(Vec3::ZERO);

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

    // Spawn player-controlled vehicle
    // commands.spawn((Vehicle::new(seg_north_east), PlayerControlled));
}

fn draw_segments(mut gizmos: Gizmos, road: Res<Road>) {
    for node in road.nodes.iter() {
        // gizmos.sphere(node.position, 0.2, Color::linear_rgb(0.0, 1.0, 0.0));
    }

    for segment in road.segments.iter() {
        let from = road.nodes.get(&segment.from);
        let to = road.nodes.get(&segment.to);

        match &segment.geometry {
            SegmentGeometry::Straight => {
                gizmos.line(from.position, to.position, Color::WHITE);
            }
            SegmentGeometry::Curved { .. } => {
                // Draw arc as a series of line segments
                const STEPS: usize = 16;
                for i in 0..STEPS {
                    let t0 = i as f32 / STEPS as f32;
                    let t1 = (i + 1) as f32 / STEPS as f32;
                    let p0 = segment.geometry.position_at(from.position, to.position, t0);
                    let p1 = segment.geometry.position_at(from.position, to.position, t1);
                    gizmos.line(p0, p1, Color::WHITE);
                }
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
