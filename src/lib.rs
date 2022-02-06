use bevy::{
    core_pipeline::ClearColor,
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
    DefaultPlugins,
};
use bevy_egui::EguiPlugin;
use bevy_polyline::PolylinePlugin;
use smooth_bevy_cameras::{
    controllers::orbit::{
        ControlEvent, OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin,
    },
    LookTransformPlugin,
};

mod error;
mod plant;
#[cfg(target_arch = "wasm32")]
mod resize;
mod ui;

pub fn run() {
    App::new().add_plugin(GamePlugin).run();
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(Msaa { samples: 4 })
            .insert_resource(ClearColor(Color::rgb(0.0, 0.02, 0.05)))
            .add_plugins(DefaultPlugins)
            .add_plugin(PolylinePlugin)
            .add_plugin(plant::PlantPlugin)
            .add_plugin(ui::UiPlugin)
            .add_plugin(EguiPlugin)
            .add_plugin(LookTransformPlugin)
            .add_plugin(OrbitCameraPlugin {
                override_input_system: true,
            })
            .add_system(input_map_system)
            .add_startup_system(setup);

        #[cfg(target_arch = "wasm32")]
        app.add_plugin(resize::ViewportPlugin);
    }
}

/// set up a simple 3D scene
fn setup(mut commands: Commands) {
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(OrbitCameraBundle::new(
        OrbitCameraController::default(),
        PerspectiveCameraBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        Vec3::new(-10.0, 5.0, -10.0),
        Vec3::new(0., 5., 0.),
    ));
}

pub fn input_map_system(
    mut events: EventWriter<ControlEvent>,
    mut mouse_wheel_reader: EventReader<MouseWheel>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mouse_buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    controllers: Query<&OrbitCameraController>,
) {
    // Can only control one camera at a time.
    let controller = if let Some(controller) = controllers.iter().next() {
        controller
    } else {
        return;
    };
    let OrbitCameraController {
        enabled,
        mouse_rotate_sensitivity,
        mouse_translate_sensitivity,
        mouse_wheel_zoom_sensitivity,
        pixels_per_line,
        ..
    } = *controller;

    if !enabled {
        return;
    }

    let mut cursor_delta = Vec2::ZERO;
    for event in mouse_motion_events.iter() {
        cursor_delta += event.delta;
    }

    if mouse_buttons.pressed(MouseButton::Middle) {
        if keyboard.pressed(KeyCode::LShift) {
            //Pan
            events.send(ControlEvent::TranslateTarget(
                mouse_translate_sensitivity * cursor_delta,
            ));
        } else {
            // Orbit
            events.send(ControlEvent::Orbit(mouse_rotate_sensitivity * cursor_delta));
        }
    }

    let mut scalar = 1.0;
    for event in mouse_wheel_reader.iter() {
        // scale the event magnitude per pixel or per line
        let scroll_amount = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / pixels_per_line,
        };
        scalar *= 1.0 - scroll_amount * mouse_wheel_zoom_sensitivity;
    }
    events.send(ControlEvent::Zoom(scalar));
}
