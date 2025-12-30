use bevy::prelude::*;
use simulation::{
    driver::{PlayerControlled, Vehicle},
    prelude::*,
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
    let north = road.nodes.alloc(Node {
        position: Vec3::new(0.0, 10.0, 0.0),
        outgoing: vec![],
    });
    let east = road.nodes.alloc(Node {
        position: Vec3::new(10.0, 0.0, 0.0),
        outgoing: vec![],
    });
    let south = road.nodes.alloc(Node {
        position: Vec3::new(0.0, -10.0, 0.0),
        outgoing: vec![],
    });
    let west = road.nodes.alloc(Node {
        position: Vec3::new(-10.0, 0.0, 0.0),
        outgoing: vec![],
    });

    // Create segments
    let seg_north_east = road.segments.alloc(Segment {
        from: north,
        to: east,
        speed_limit: 5.0,
    });
    let seg_east_south = road.segments.alloc(Segment {
        from: east,
        to: south,
        speed_limit: 5.0,
    });
    let seg_south_west = road.segments.alloc(Segment {
        from: south,
        to: west,
        speed_limit: 5.0,
    });
    let seg_west_north = road.segments.alloc(Segment {
        from: west,
        to: north,
        speed_limit: 5.0,
    });

    // Wire up outgoing connections
    road.nodes.get_mut(&north).outgoing.push(seg_north_east);
    road.nodes.get_mut(&east).outgoing.push(seg_east_south);
    road.nodes.get_mut(&south).outgoing.push(seg_south_west);
    road.nodes.get_mut(&west).outgoing.push(seg_west_north);

    commands.insert_resource(road);

    commands.spawn(Vehicle::new(seg_west_north));
    commands.spawn(Vehicle::new(seg_south_west));
    commands.spawn(Vehicle::new(seg_east_south));

    // Spawn player-controlled vehicle
    commands.spawn((Vehicle::new(seg_north_east), PlayerControlled));
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
