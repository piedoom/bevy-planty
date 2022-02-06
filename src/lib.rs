use bevy::{core_pipeline::ClearColor, prelude::*, DefaultPlugins};
use bevy_egui::EguiPlugin;
use bevy_polyline::PolylinePlugin;
use smooth_bevy_cameras::{
    controllers::orbit::{OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin},
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
            .add_plugin(OrbitCameraPlugin::default())
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
