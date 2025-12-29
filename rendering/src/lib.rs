use bevy::prelude::*;
use simulation::prelude::*;
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
        .add_systems(Update, (draw_vehicles, draw_segments))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Light
    commands.spawn((PointLight::default(), Transform::from_xyz(4.0, 8.0, 4.0)));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scale: 0.05,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub fn test_intersection(mut commands: Commands) {
    let mut road = Road::default();

    // Create nodes
    let left = road.nodes.alloc(Node {
        position: Vec3::new(-10.0, 0.0, 0.0),
        outgoing: vec![],
    });
    let center = road.nodes.alloc(Node {
        position: Vec3::ZERO,
        outgoing: vec![],
    });
    let right = road.nodes.alloc(Node {
        position: Vec3::new(10.0, 0.0, 0.0),
        outgoing: vec![],
    });

    // Create segments
    let seg1 = road.segments.alloc(Segment {
        from: left,
        to: center,
    });
    let seg2 = road.segments.alloc(Segment {
        from: center,
        to: right,
    });

    // Wire up outgoing connections
    road.nodes.get_mut(&left).outgoing.push(seg1);
    road.nodes.get_mut(&center).outgoing.push(seg2);

    commands.insert_resource(road);

    // Spawn a test vehicle
    commands.spawn((
        Vehicle { speed: 2.0 },
        OnSegment {
            segment: seg1,
            progress: 0.0,
        },
    ));
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

fn draw_vehicles(mut gizmos: Gizmos, vehicles: Query<&OnSegment>, road: Res<Road>) {
    for on_segment in &vehicles {
        let segment = road.segments.get(&on_segment.segment);
        let from = road.nodes.get(&segment.from);
        let to = road.nodes.get(&segment.to);

        let position = from.position.lerp(to.position, on_segment.progress);

        gizmos.sphere(position, 0.5, Color::linear_rgb(1.0, 0.0, 0.0));
    }
}
