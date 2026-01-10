//! Simulation module - GPU compute pipeline for particle physics.

mod setup;
mod systems;

use bevy::{
    prelude::*,
    render::{
        render_graph::{RenderGraph, RenderLabel},
        Render, RenderApp, RenderSet,
    },
};

pub use setup::{ParticleBuffer, SimParamsBuffer, PARTICLE_COUNT};

/// Plugin that manages the GPU compute pipeline for particle simulation.
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        // Set up render app with compute pipeline
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(Render, systems::prepare_bind_group.in_set(RenderSet::Prepare))
            .add_systems(Render, systems::queue_compute.in_set(RenderSet::Queue));

        // Add compute node to render graph
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(PhysicsLabel, systems::PhysicsNode::default());
        render_graph.add_node_edge(PhysicsLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        // Initialize buffers and pipeline in the render app (where RenderDevice is available)
        render_app.init_resource::<ParticleBuffer>();
        render_app.init_resource::<SimParamsBuffer>();
        render_app.init_resource::<systems::PhysicsPipeline>();
    }
}

/// Label for the physics compute node in the render graph.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PhysicsLabel;
