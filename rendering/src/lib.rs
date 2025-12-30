use bevy::prelude::*;
use simulation::{
    driver::{PlayerControlled, Vehicle},
    Road, SimulationPlugin,
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
        .add_systems(Update, (draw_vehicles, draw_segments, player_input))
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
    let north_entry = road.add_node(Vec3::new(0.0, 10.0, 0.0));
    let north_exit = road.add_node(Vec3::new(0.0, 10.0, 0.0));
    let east_entry = road.add_node(Vec3::new(10.0, 0.0, 0.0));
    let east_exit = road.add_node(Vec3::new(10.0, 0.0, 0.0));
    let south_entry = road.add_node(Vec3::new(0.0, -10.0, 0.0));
    let south_exit = road.add_node(Vec3::new(0.0, -10.0, 0.0));
    let west_entry = road.add_node(Vec3::new(-10.0, 0.0, 0.0));
    let west_exit = road.add_node(Vec3::new(-10.0, 0.0, 0.0));
    let center = road.add_node(Vec3::ZERO);

    // Create segments (incoming/outgoing wired automatically)
    road.add_segment(north_entry, center, 5.0);
    road.add_segment(center, north_exit, 5.0);
    road.add_segment(east_entry, center, 5.0);
    road.add_segment(center, east_exit, 5.0);
    road.add_segment(south_entry, center, 5.0);
    road.add_segment(center, south_exit, 5.0);
    road.add_segment(west_entry, center, 5.0);
    road.add_segment(center, west_exit, 5.0);

    commands.insert_resource(road);

    // Spawn player-controlled vehicle
    // commands.spawn((Vehicle::new(seg_north_east), PlayerControlled));
}

fn draw_segments(mut gizmos: Gizmos, road: Res<Road>) {
    for node in road.nodes.iter() {
        gizmos.sphere(node.position, 0.2, Color::linear_rgb(0.0, 1.0, 0.0));
    }

    for segment in road.segments.iter() {
        let from = road.nodes.get(&segment.from);
        let to = road.nodes.get(&segment.to);

        gizmos.line(from.position, to.position, Color::WHITE);
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

        let position = from.position.lerp(to.position, vehicle.progress);

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
