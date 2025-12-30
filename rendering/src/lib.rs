use bevy::prelude::*;
use simulation::{
    driver::{PlayerControlled, Vehicle},
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
    let north = road.add_edge_node(Vec3::new(0.0, 20.0, 0.0));
    let south = road.add_edge_node(Vec3::new(0.0, -20.0, 0.0));
    let east = road.add_edge_node(Vec3::new(20.0, 0.0, 0.0));
    let west = road.add_edge_node(Vec3::new(-20.0, 0.0, 0.0));
    let south_east = road.add_edge_node(Vec3::new(20.0, -20.0, 0.0));
    let center = road.add_node(Vec3::ZERO);

    // Create segments (incoming/outgoing wired automatically)
    road.add_bidirectional(north, center, 5.0);
    road.add_bidirectional(south, center, 5.0);
    road.add_bidirectional(east, center, 5.0);
    road.add_bidirectional(west, center, 5.0);
    road.add_bidirectional(south_east, center, 5.0);

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

        let color = if is_player.is_some() {
            Color::linear_rgb(0.2, 0.5, 1.0) // Blue for player
        } else {
            Color::linear_rgb(1.0, 0.0, 0.0) // Red for AI
        };

        gizmos.sphere(position, 0.5, color);
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
