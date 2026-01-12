//! Simulation module - GPU compute pipeline for SPH particle physics.

mod setup;
mod systems;

use bevy::{
    prelude::*,
    render::{
        render_graph::{RenderGraph, RenderLabel},
        Render, RenderApp, RenderSet,
    },
};

pub use setup::{
    CellCountsBuffer, CellOffsetsBuffer, GridParamsBuffer, IndexBuffer, 
    ParticleBuffer, SimParamsBuffer, PARTICLE_COUNT,
};

/// Plugin that manages the GPU compute pipeline for SPH particle simulation.
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(Render, systems::prepare_bind_group.in_set(RenderSet::Prepare))
            .add_systems(Render, systems::queue_compute.in_set(RenderSet::Queue));

        // Add SPH compute node to render graph
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(SphPhysicsLabel, systems::SphPhysicsNode::default());
        render_graph.add_node_edge(SphPhysicsLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        // Initialize all buffers in the render app
        render_app.init_resource::<ParticleBuffer>();
        render_app.init_resource::<SimParamsBuffer>();
        render_app.init_resource::<GridParamsBuffer>();
        render_app.init_resource::<IndexBuffer>();
        render_app.init_resource::<CellCountsBuffer>();
        render_app.init_resource::<CellOffsetsBuffer>();
        // Initialize compute pipelines
        render_app.init_resource::<systems::SphPipelines>();
    }
}

/// Label for the SPH physics compute node in the render graph.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SphPhysicsLabel;

// Keep old label for compatibility
pub type PhysicsLabel = SphPhysicsLabel;
