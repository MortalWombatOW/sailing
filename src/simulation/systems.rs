//! Compute shader systems for SPH physics simulation.
//! 
//! Pipeline stages:
//! 1. Cell ID calculation
//! 2. Bitonic Sort (multiple passes)
//! 3. Build Grid (clear + build)
//! 4. Density calculation
//! 5. Force calculation
//! 6. Position integration

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraphContext},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};

use super::setup::{
    CellRangeBuffer, GridParamsBuffer, IndexBuffer, ParticleBuffer, SimParamsBuffer,
    SortParamsBuffer, PARTICLE_COUNT,
};

// ==================== Pipeline Resources ====================

/// All compute pipelines for SPH simulation
#[derive(Resource)]
pub struct SphPipelines {
    pub cell_id: CachedComputePipelineId,
    pub sort: CachedComputePipelineId,
    pub clear_grid: CachedComputePipelineId,
    pub build_grid: CachedComputePipelineId,
    pub density: CachedComputePipelineId,
    pub forces: CachedComputePipelineId,
    pub physics: CachedComputePipelineId,
    // Bind group layouts
    pub cell_id_layout: BindGroupLayout,
    pub sort_layout: BindGroupLayout,
    pub grid_layout: BindGroupLayout,
    pub density_layout: BindGroupLayout,
    pub physics_layout: BindGroupLayout,
}

impl FromWorld for SphPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // Cell ID layout: particles (rw), grid params (uniform)
        let cell_id_layout = render_device.create_bind_group_layout(
            Some("CellID Layout"),
            &[
                storage_buffer_entry(0, false), // particles rw
                uniform_buffer_entry(1),        // grid params
            ],
        );

        // Sort layout: particles (read), indices (rw), sort params (uniform)
        let sort_layout = render_device.create_bind_group_layout(
            Some("Sort Layout"),
            &[
                storage_buffer_entry(0, true),  // particles read
                storage_buffer_entry(1, false), // indices rw
                uniform_buffer_entry(2),        // sort params
            ],
        );

        // Grid layout: particles (read), indices (read), cell_ranges (rw), grid params
        let grid_layout = render_device.create_bind_group_layout(
            Some("Grid Layout"),
            &[
                storage_buffer_entry(0, true),  // particles read
                storage_buffer_entry(1, true),  // indices read
                storage_buffer_entry(2, false), // cell_ranges rw
                uniform_buffer_entry(3),        // grid params
            ],
        );

        // Density/Forces layout (simplified for brute force): particles (rw), sim params
        let density_layout = render_device.create_bind_group_layout(
            Some("Density/Forces Layout"),
            &[
                storage_buffer_entry(0, false), // particles rw
                uniform_buffer_entry(1),        // sim params
            ],
        );

        // Physics layout: particles (rw), sim params (same as density now)
        let physics_layout = render_device.create_bind_group_layout(
            Some("Physics Layout"),
            &[
                storage_buffer_entry(0, false), // particles rw
                uniform_buffer_entry(1),        // sim params
            ],
        );

        // Load shaders
        let cell_id_shader = asset_server.load("shaders/cell_id.wgsl");
        let sort_shader = asset_server.load("shaders/sort.wgsl");
        let build_grid_shader = asset_server.load("shaders/build_grid.wgsl");
        let density_shader = asset_server.load("shaders/density.wgsl");
        let forces_shader = asset_server.load("shaders/forces.wgsl");
        let physics_shader = asset_server.load("shaders/physics.wgsl");

        // Create pipelines
        let cell_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("CellID Pipeline".into()),
            layout: vec![cell_id_layout.clone()],
            shader: cell_id_shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let sort = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Sort Pipeline".into()),
            layout: vec![sort_layout.clone()],
            shader: sort_shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let clear_grid = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Clear Grid Pipeline".into()),
            layout: vec![grid_layout.clone()],
            shader: build_grid_shader.clone(),
            shader_defs: vec![],
            entry_point: "clear_cells".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let build_grid = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Build Grid Pipeline".into()),
            layout: vec![grid_layout.clone()],
            shader: build_grid_shader,
            shader_defs: vec![],
            entry_point: "build_grid".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let density = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Density Pipeline".into()),
            layout: vec![density_layout.clone()],
            shader: density_shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let forces = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Forces Pipeline".into()),
            layout: vec![density_layout.clone()],
            shader: forces_shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let physics = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Physics Pipeline".into()),
            layout: vec![physics_layout.clone()],
            shader: physics_shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        Self {
            cell_id,
            sort,
            clear_grid,
            build_grid,
            density,
            forces,
            physics,
            cell_id_layout,
            sort_layout,
            grid_layout,
            density_layout,
            physics_layout,
        }
    }
}

// Helper functions for bind group layout entries
fn storage_buffer_entry(binding: u32, read_only: bool) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_buffer_entry(binding: u32) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

// ==================== Bind Groups ====================

#[derive(Resource)]
pub struct SphBindGroups {
    pub cell_id: BindGroup,
    pub sort: BindGroup,
    pub grid: BindGroup,
    pub density: BindGroup,
    pub physics: BindGroup,
}

/// Prepare all bind groups
pub fn prepare_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Option<Res<SphPipelines>>,
    particles: Option<Res<ParticleBuffer>>,
    indices: Option<Res<IndexBuffer>>,
    cell_ranges: Option<Res<CellRangeBuffer>>,
    grid_params: Option<Res<GridParamsBuffer>>,
    sim_params: Option<Res<SimParamsBuffer>>,
    sort_params: Option<Res<SortParamsBuffer>>,
) {
    let (
        Some(pipelines),
        Some(particles),
        Some(indices),
        Some(cell_ranges),
        Some(grid_params),
        Some(sim_params),
        Some(sort_params),
    ) = (
        pipelines,
        particles,
        indices,
        cell_ranges,
        grid_params,
        sim_params,
        sort_params,
    )
    else {
        return;
    };

    let cell_id = render_device.create_bind_group(
        Some("CellID BindGroup"),
        &pipelines.cell_id_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: grid_params.0.as_entire_binding(),
            },
        ],
    );

    let sort = render_device.create_bind_group(
        Some("Sort BindGroup"),
        &pipelines.sort_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: indices.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: sort_params.0.as_entire_binding(),
            },
        ],
    );

    let grid = render_device.create_bind_group(
        Some("Grid BindGroup"),
        &pipelines.grid_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: indices.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: cell_ranges.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: grid_params.0.as_entire_binding(),
            },
        ],
    );

    let density = render_device.create_bind_group(
        Some("Density BindGroup"),
        &pipelines.density_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: sim_params.0.as_entire_binding(),
            },
        ],
    );

    let physics = render_device.create_bind_group(
        Some("Physics BindGroup"),
        &pipelines.physics_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: sim_params.0.as_entire_binding(),
            },
        ],
    );

    commands.insert_resource(SphBindGroups {
        cell_id,
        sort,
        grid,
        density,
        physics,
    });
}

pub fn queue_compute() {
    // Handled by render graph node
}

// ==================== Render Graph Node ====================

/// SPH physics compute node - runs all stages in sequence
#[derive(Default)]
pub struct SphPhysicsNode;

impl render_graph::Node for SphPhysicsNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        
        let Some(pipelines) = world.get_resource::<SphPipelines>() else {
            return Ok(());
        };
        
        let Some(bind_groups) = world.get_resource::<SphBindGroups>() else {
            return Ok(());
        };

        // Get all pipelines (if any aren't ready, skip this frame)
        let Some(cell_id_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.cell_id) else {
            return Ok(());
        };
        let Some(density_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.density) else {
            return Ok(());
        };
        let Some(forces_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.forces) else {
            return Ok(());
        };
        let Some(physics_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.physics) else {
            return Ok(());
        };

        let workgroup_count = (PARTICLE_COUNT as u32 + 63) / 64;

        // Stage 1: Calculate cell IDs
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("CellID Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(cell_id_pipeline);
            pass.set_bind_group(0, &bind_groups.cell_id, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Stage 2-3: Skip sorting for now (simplified version)
        // TODO: Implement full bitonic sort - for now use brute force neighbor search
        
        // Stage 4: Density calculation
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Density Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(density_pipeline);
            pass.set_bind_group(0, &bind_groups.density, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Stage 5: Force calculation
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Forces Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(forces_pipeline);
            pass.set_bind_group(0, &bind_groups.density, &[]); // Same layout
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Stage 6: Position integration
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Physics Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(physics_pipeline);
            pass.set_bind_group(0, &bind_groups.physics, &[]);
            pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        Ok(())
    }
}

// Keep old names for compatibility with mod.rs
pub type PhysicsPipeline = SphPipelines;
pub type PhysicsNode = SphPhysicsNode;
pub fn prepare_bind_group(
    commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Option<Res<SphPipelines>>,
    particles: Option<Res<ParticleBuffer>>,
    indices: Option<Res<IndexBuffer>>,
    cell_ranges: Option<Res<CellRangeBuffer>>,
    grid_params: Option<Res<GridParamsBuffer>>,
    sim_params: Option<Res<SimParamsBuffer>>,
    sort_params: Option<Res<SortParamsBuffer>>,
) {
    prepare_bind_groups(
        commands,
        render_device,
        pipelines,
        particles,
        indices,
        cell_ranges,
        grid_params,
        sim_params,
        sort_params,
    );
}
