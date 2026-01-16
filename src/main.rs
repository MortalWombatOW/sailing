//! Sailing SPH Simulation - Main Entry
//!
//! A 2.5D top-down sailing simulation using GPU-accelerated SPH physics.

use bevy::prelude::*;
use sailing::render::ParticleRenderPlugin;
use sailing::simulation::SimulationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sailing SPH Sim".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SimulationPlugin)
        .add_plugins(ParticleRenderPlugin)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, log_frame)
        .run();
}

/// Set up the 2D main camera
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.insert_resource(FrameCounter(0));
}

/// Frame counter for logging
#[derive(Resource)]
struct FrameCounter(u32);

/// Log every N frames
fn log_frame(mut counter: ResMut<FrameCounter>) {
    counter.0 += 1;
    if counter.0 % 60 == 0 {
        info!("Frame {}: Simulation running...", counter.0);
    }
}
