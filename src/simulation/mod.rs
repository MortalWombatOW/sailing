//! Simulation module - GPU compute pipeline for SPH particle physics.

pub mod input;
mod physics_config;
mod scenarios;
mod setup;
mod systems;

pub use physics_config::{InteractionProfile, InteractionTable, MaterialType, default_interaction_table};
pub use input::SailControl;

use bevy::{
    prelude::*,
    render::{
        render_graph::{RenderGraph, RenderLabel},
        render_resource::BufferUsages,
        renderer::RenderQueue,
        Extract, Render, RenderApp, RenderSet,
    },
};
use crate::resources::SimParams;

pub use setup::{
    BondBuffer, CellCountsBuffer, CellOffsetsBuffer, ForceBuffer, GridParamsBuffer, IndexBuffer,
    InteractionTableBuffer, ParticleBuffer, SimParamsBuffer, BOND_COUNT, PARTICLE_COUNT,
};

/// Extracted sail angle for the render app
#[derive(Resource, Default)]
pub struct ExtractedSailAngle(pub f32);

/// Extract sail angle from main app to render app
fn extract_sail_angle(
    mut commands: Commands,
    sail_control: Extract<Option<Res<SailControl>>>,
) {
    if let Some(sail) = sail_control.as_ref() {
        commands.insert_resource(ExtractedSailAngle(sail.angle));
    }
}

/// Update SimParams buffer with sail angle from main app
fn update_sim_params_buffer(
    render_queue: Res<RenderQueue>,
    sim_params_buffer: Option<Res<SimParamsBuffer>>,
    sail_angle: Option<Res<ExtractedSailAngle>>,
) {
    let (Some(buffer), Some(angle)) = (sim_params_buffer, sail_angle) else {
        return;
    };
    
    // Create updated SimParams with current sail angle
    let mut params = SimParams::default();
    params.rudder_angle = angle.0;  // Using rudder_angle field for sail control
    
    // Write to buffer
    render_queue.write_buffer(&buffer.0, 0, bytemuck::bytes_of(&params));
}

/// Plugin that manages the GPU compute pipeline for SPH particle simulation.
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        // Main app: input handling
        app.init_resource::<SailControl>()
            .add_systems(Update, input::handle_sail_input);
        
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ExtractedSailAngle>()
            .add_systems(bevy::render::ExtractSchedule, extract_sail_angle)
            .add_systems(Render, update_sim_params_buffer.in_set(RenderSet::Prepare).before(systems::prepare_bind_group))
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
        render_app.init_resource::<BondBuffer>();
        render_app.init_resource::<ForceBuffer>();
        render_app.init_resource::<InteractionTableBuffer>();
        // Initialize compute pipelines
        render_app.init_resource::<systems::SphPipelines>();
    }
}

/// Label for the SPH physics compute node in the render graph.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SphPhysicsLabel;

// Keep old label for compatibility
pub type PhysicsLabel = SphPhysicsLabel;
