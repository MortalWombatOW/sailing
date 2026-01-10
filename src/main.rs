//! Sailing SPH Simulation - Main Entry
//!
//! A 2.5D top-down sailing simulation using GPU-accelerated SPH physics.

mod render;
mod resources;
mod simulation;

use bevy::prelude::*;
use render::ParticleRenderPlugin;
use simulation::SimulationPlugin;

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
        .run();
}

/// Set up the 2D main camera
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
