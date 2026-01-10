//! Compute shader systems for physics simulation.

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraphContext},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};

use super::{ParticleBuffer, SimParamsBuffer, PARTICLE_COUNT};

/// Compute pipeline resource
#[derive(Resource)]
pub struct PhysicsPipeline {
    pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for PhysicsPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(
            Some("Physics Bind Group Layout"),
            &[
                // Particle buffer (read-write storage)
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // SimParams uniform
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );

        // Load and create compute pipeline
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/physics.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Physics Compute Pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// Bind group for physics compute
#[derive(Resource)]
pub struct PhysicsBindGroup(pub BindGroup);

/// System to prepare the bind group each frame
pub fn prepare_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<PhysicsPipeline>,
    particle_buffer: Option<Res<ParticleBuffer>>,
    params_buffer: Option<Res<SimParamsBuffer>>,
) {
    let (Some(particle_buffer), Some(params_buffer)) = (particle_buffer, params_buffer) else {
        return;
    };

    let bind_group = render_device.create_bind_group(
        Some("Physics Bind Group"),
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particle_buffer.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: params_buffer.0.as_entire_binding(),
            },
        ],
    );

    commands.insert_resource(PhysicsBindGroup(bind_group));
}

/// System to queue compute work
pub fn queue_compute() {
    // Compute work is handled by the render graph node
}

/// Render graph node for physics compute
#[derive(Default)]
pub struct PhysicsNode;

impl render_graph::Node for PhysicsNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let physics_pipeline = world.resource::<PhysicsPipeline>();

        let Some(bind_group) = world.get_resource::<PhysicsBindGroup>() else {
            return Ok(());
        };

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(physics_pipeline.pipeline) else {
            return Ok(());
        };

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Physics Compute Pass"),
                timestamp_writes: None,
            });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group.0, &[]);

        // Dispatch workgroups (64 threads per workgroup)
        let workgroup_count = (PARTICLE_COUNT as u32 + 63) / 64;
        pass.dispatch_workgroups(workgroup_count, 1, 1);

        Ok(())
    }
}
