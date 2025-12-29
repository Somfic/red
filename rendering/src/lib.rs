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
    let center = commands
        .spawn(Node {
            position: Vec3::ZERO,
        })
        .id();

    let left = commands
        .spawn(Node {
            position: Vec3::new(-10.0, 0.0, 0.0),
        })
        .id();

    let right = commands
        .spawn(Node {
            position: Vec3::new(10.0, 0.0, 0.0),
        })
        .id();

    let segment = commands
        .spawn(Segment {
            from: left,
            to: center,
        })
        .id();

    commands.spawn(Segment {
        from: center,
        to: right,
    });

    commands.spawn((
        Vehicle { speed: 2.0 },
        OnSegment {
            segment,
            progress: 0.0,
        },
    ));
}

fn draw_segments(mut gizmos: Gizmos, segments: Query<&Segment>, nodes: Query<&Node>) {
    for node in &nodes {
        gizmos.sphere(node.position, 0.2, Color::linear_rgb(0.0, 1.0, 0.0));
    }

    for segment in &segments {
        let Ok(from) = nodes.get(segment.from) else {
            continue;
        };
        let Ok(to) = nodes.get(segment.to) else {
            continue;
        };

        gizmos.line(from.position, to.position, Color::WHITE);
    }
}

fn draw_vehicles(
    mut gizmos: Gizmos,
    vehicles: Query<(&OnSegment,)>,
    segments: Query<&Segment>,
    nodes: Query<&Node>,
) {
    for (on_segment,) in &vehicles {
        let segment = on_segment.segment;
        let Ok(segment) = segments.get(segment) else {
            continue;
        };
        let Ok(from) = nodes.get(segment.from) else {
            continue;
        };
        let Ok(to) = nodes.get(segment.to) else {
            continue;
        };

        let position = from.position.lerp(to.position, on_segment.progress);

        gizmos.sphere(position, 0.5, Color::linear_rgb(1.0, 0.0, 0.0));
    }
}
