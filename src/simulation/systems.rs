//! Compute shader systems for SPH physics simulation.
//! 
//! Pipeline stages (Counting Sort):
//! 1. Cell ID calculation
//! 2. Clear cell counts
//! 3. Count cells
//! 4. Prefix sum
//! 5. Scatter sort
//! 6. Density calculation
//! 7. Force calculation
//! 8. Position integration

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraphContext},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};

use super::setup::{
    BondBuffer, CellCountsBuffer, CellOffsetsBuffer, ForceBuffer, GridParamsBuffer, IndexBuffer,
    ParticleBuffer, SimParamsBuffer, BOND_COUNT, PARTICLE_COUNT,
};
use crate::resources::GridParams;

// ==================== Pipeline Resources ====================

/// All compute pipelines for SPH simulation
#[derive(Resource)]
pub struct SphPipelines {
    pub cell_id: CachedComputePipelineId,
    pub clear_counts: CachedComputePipelineId,
    pub count_cells: CachedComputePipelineId,
    pub prefix_sum: CachedComputePipelineId,
    pub scatter: CachedComputePipelineId,
    pub density: CachedComputePipelineId,
    pub forces: CachedComputePipelineId,
    pub bonds: CachedComputePipelineId,
    pub physics: CachedComputePipelineId,
    // Bind group layouts
    pub cell_id_layout: BindGroupLayout,
    pub count_layout: BindGroupLayout,
    pub prefix_layout: BindGroupLayout,
    pub scatter_layout: BindGroupLayout,
    pub density_layout: BindGroupLayout,
    pub bonds_layout: BindGroupLayout,
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

        // Count layout: particles (read), cell_counts (rw), grid params
        let count_layout = render_device.create_bind_group_layout(
            Some("Count Layout"),
            &[
                storage_buffer_entry(0, true),  // particles read
                storage_buffer_entry(1, false), // cell_counts rw (atomic)
                uniform_buffer_entry(2),        // grid params
            ],
        );

        // Prefix sum layout: cell_counts (read), cell_offsets (rw), grid params
        let prefix_layout = render_device.create_bind_group_layout(
            Some("Prefix Sum Layout"),
            &[
                storage_buffer_entry(0, true),  // cell_counts read
                storage_buffer_entry(1, false), // cell_offsets rw
                uniform_buffer_entry(2),        // grid params
            ],
        );

        // Scatter layout: particles (read), cell_offsets (rw atomic), sorted_indices (rw)
        let scatter_layout = render_device.create_bind_group_layout(
            Some("Scatter Layout"),
            &[
                storage_buffer_entry(0, true),  // particles read
                storage_buffer_entry(1, false), // cell_offsets rw (atomic)
                storage_buffer_entry(2, false), // sorted_indices rw
            ],
        );

        // Density/Forces layout: particles, indices, cell_offsets, grid, sim params
        let density_layout = render_device.create_bind_group_layout(
            Some("Density/Forces Layout"),
            &[
                storage_buffer_entry(0, false), // particles rw
                storage_buffer_entry(1, true),  // indices read
                storage_buffer_entry(2, true),  // cell_offsets read
                uniform_buffer_entry(3),        // grid params
                uniform_buffer_entry(4),        // sim params
            ],
        );

        // Bonds layout: particles (rw), bonds (rw), forces (atomic rw)
        let bonds_layout = render_device.create_bind_group_layout(
            Some("Bonds Layout"),
            &[
                storage_buffer_entry(0, false), // particles rw
                storage_buffer_entry(1, false), // bonds rw
                storage_buffer_entry(2, false), // forces atomic rw
            ],
        );

        // Physics layout: particles (rw), sim params, forces (atomic rw)
        let physics_layout = render_device.create_bind_group_layout(
            Some("Physics Layout"),
            &[
                storage_buffer_entry(0, false), // particles rw
                uniform_buffer_entry(1),        // sim params
                storage_buffer_entry(2, false), // forces atomic rw
            ],
        );

        // Load shaders
        let cell_id_shader = asset_server.load("shaders/cell_id.wgsl");
        let count_cells_shader = asset_server.load("shaders/count_cells.wgsl");
        let prefix_sum_shader = asset_server.load("shaders/prefix_sum.wgsl");
        let scatter_shader = asset_server.load("shaders/scatter_sort.wgsl");
        let density_shader = asset_server.load("shaders/density.wgsl");
        let forces_shader = asset_server.load("shaders/forces.wgsl");
        let bonds_shader = asset_server.load("shaders/bonds.wgsl");
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

        let clear_counts = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Clear Counts Pipeline".into()),
            layout: vec![count_layout.clone()],
            shader: count_cells_shader.clone(),
            shader_defs: vec![],
            entry_point: "clear_counts".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let count_cells = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Count Cells Pipeline".into()),
            layout: vec![count_layout.clone()],
            shader: count_cells_shader,
            shader_defs: vec![],
            entry_point: "count_cells".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let prefix_sum = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Prefix Sum Pipeline".into()),
            layout: vec![prefix_layout.clone()],
            shader: prefix_sum_shader,
            shader_defs: vec![],
            entry_point: "prefix_sum".into(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        let scatter = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Scatter Pipeline".into()),
            layout: vec![scatter_layout.clone()],
            shader: scatter_shader,
            shader_defs: vec![],
            entry_point: "scatter".into(),
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

        let bonds = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Bonds Pipeline".into()),
            layout: vec![bonds_layout.clone()],
            shader: bonds_shader,
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
            clear_counts,
            count_cells,
            prefix_sum,
            scatter,
            density,
            forces,
            bonds,
            physics,
            cell_id_layout,
            count_layout,
            prefix_layout,
            scatter_layout,
            density_layout,
            bonds_layout,
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
    pub count: BindGroup,
    pub prefix: BindGroup,
    pub scatter: BindGroup,
    pub density: BindGroup,
    pub bonds: BindGroup,
    pub physics: BindGroup,
}

/// Prepare all bind groups
fn prepare_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Option<Res<SphPipelines>>,
    particles: Option<Res<ParticleBuffer>>,
    indices: Option<Res<IndexBuffer>>,
    cell_counts: Option<Res<CellCountsBuffer>>,
    cell_offsets: Option<Res<CellOffsetsBuffer>>,
    grid_params: Option<Res<GridParamsBuffer>>,
    sim_params: Option<Res<SimParamsBuffer>>,
    bond_buffer: Option<Res<BondBuffer>>,
    force_buffer: Option<Res<ForceBuffer>>,
) {
    let (Some(pipelines), Some(particles), Some(indices), Some(cell_counts), 
         Some(cell_offsets), Some(grid_params), Some(sim_params), Some(bond_buffer), Some(force_buffer)) = 
        (pipelines, particles, indices, cell_counts, cell_offsets, grid_params, sim_params, bond_buffer, force_buffer) 
    else {
        return;
    };

    // Cell ID bind group
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

    // Count bind group
    let count = render_device.create_bind_group(
        Some("Count BindGroup"),
        &pipelines.count_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: cell_counts.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: grid_params.0.as_entire_binding(),
            },
        ],
    );

    // Prefix sum bind group
    let prefix = render_device.create_bind_group(
        Some("Prefix Sum BindGroup"),
        &pipelines.prefix_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: cell_counts.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: cell_offsets.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: grid_params.0.as_entire_binding(),
            },
        ],
    );

    // Scatter bind group
    let scatter = render_device.create_bind_group(
        Some("Scatter BindGroup"),
        &pipelines.scatter_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: cell_offsets.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: indices.0.as_entire_binding(),
            },
        ],
    );

    // Density/Forces bind group
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
                resource: indices.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: cell_offsets.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: grid_params.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: sim_params.0.as_entire_binding(),
            },
        ],
    );

    // Bonds bind group
    let bonds = render_device.create_bind_group(
        Some("Bonds BindGroup"),
        &pipelines.bonds_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: particles.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: bond_buffer.0.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: force_buffer.0.as_entire_binding(),
            },
        ],
    );

    // Physics bind group
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
            BindGroupEntry {
                binding: 2,
                resource: force_buffer.0.as_entire_binding(),
            },
        ],
    );

    commands.insert_resource(SphBindGroups {
        cell_id,
        count,
        prefix,
        scatter,
        density,
        bonds,
        physics,
    });
}

pub fn queue_compute() {
    // Nothing needed here currently
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
        // Also need Force Buffer to clear it
        let Some(force_buffer) = world.get_resource::<ForceBuffer>() else {
            return Ok(());
        };

        // Get all pipelines (if any aren't ready, skip this frame)
        let Some(cell_id_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.cell_id) else {
            return Ok(());
        };
        let Some(clear_counts_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.clear_counts) else {
            return Ok(());
        };
        let Some(count_cells_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.count_cells) else {
            return Ok(());
        };
        let Some(prefix_sum_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.prefix_sum) else {
            return Ok(());
        };
        let Some(scatter_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.scatter) else {
            return Ok(());
        };
        let Some(density_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.density) else {
            return Ok(());
        };
        let Some(forces_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.forces) else {
            return Ok(());
        };
        let Some(bonds_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.bonds) else {
            return Ok(());
        };
        let Some(physics_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.physics) else {
            return Ok(());
        };

        let particle_workgroup_count = (PARTICLE_COUNT as u32 + 63) / 64;
        let bond_workgroup_count = (BOND_COUNT as u32 + 255) / 256;
        let grid_params = GridParams::default();
        let total_cells = grid_params.grid_width * grid_params.grid_height;
        let cell_workgroup_count = (total_cells + 255) / 256;

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
            pass.dispatch_workgroups(particle_workgroup_count, 1, 1);
        }

        // Stage 2: Clear cell counts
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Clear Counts Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(clear_counts_pipeline);
            pass.set_bind_group(0, &bind_groups.count, &[]);
            pass.dispatch_workgroups(cell_workgroup_count, 1, 1);
        }

        // Stage 3: Count particles per cell
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Count Cells Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(count_cells_pipeline);
            pass.set_bind_group(0, &bind_groups.count, &[]);
            pass.dispatch_workgroups(particle_workgroup_count, 1, 1);
        }

        // Stage 4: Prefix sum (single thread for simplicity)
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Prefix Sum Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(prefix_sum_pipeline);
            pass.set_bind_group(0, &bind_groups.prefix, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        // Stage 5: Scatter particles to sorted positions
        // NOTE: This modifies cell_offsets via atomicAdd, so we need to restore them after
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Scatter Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(scatter_pipeline);
            pass.set_bind_group(0, &bind_groups.scatter, &[]);
            pass.dispatch_workgroups(particle_workgroup_count, 1, 1);
        }

        // Stage 5b: Re-run prefix sum to restore cell_offsets for neighbor lookup
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Prefix Sum Restore Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(prefix_sum_pipeline);
            pass.set_bind_group(0, &bind_groups.prefix, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        // Stage 6: Density calculation
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Density Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(density_pipeline);
            pass.set_bind_group(0, &bind_groups.density, &[]);
            pass.dispatch_workgroups(particle_workgroup_count, 1, 1);
        }

        // Stage 7: Force calculation
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Forces Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(forces_pipeline);
            pass.set_bind_group(0, &bind_groups.density, &[]);
            pass.dispatch_workgroups(particle_workgroup_count, 1, 1);
        }

        // Stage 7.5: Bond Force calculation
        // IMPORTANT: Clear Force Buffer first!
        render_context.command_encoder().clear_buffer(&force_buffer.0, 0, None);
        
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Bonds Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(bonds_pipeline);
            pass.set_bind_group(0, &bind_groups.bonds, &[]);
            pass.dispatch_workgroups(bond_workgroup_count, 1, 1);
        }

        // Stage 8: Position integration
        {
            let mut pass = render_context.command_encoder().begin_compute_pass(
                &ComputePassDescriptor {
                    label: Some("Physics Pass"),
                    timestamp_writes: None,
                },
            );
            pass.set_pipeline(physics_pipeline);
            pass.set_bind_group(0, &bind_groups.physics, &[]);
            pass.dispatch_workgroups(particle_workgroup_count, 1, 1);
        }

        Ok(())
    }
}

// Public system wrapper
pub fn prepare_bind_group(
    commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Option<Res<SphPipelines>>,
    particles: Option<Res<ParticleBuffer>>,
    indices: Option<Res<IndexBuffer>>,
    cell_counts: Option<Res<CellCountsBuffer>>,
    cell_offsets: Option<Res<CellOffsetsBuffer>>,
    grid_params: Option<Res<GridParamsBuffer>>,
    sim_params: Option<Res<SimParamsBuffer>>,
    bond_buffer: Option<Res<BondBuffer>>,
    force_buffer: Option<Res<ForceBuffer>>,
) {
    prepare_bind_groups(
        commands,
        render_device,
        pipelines,
        particles,
        indices,
        cell_counts,
        cell_offsets,
        grid_params,
        sim_params,
        bond_buffer,
        force_buffer,
    );
}

// Keep old names for compatibility with mod.rs
pub type PhysicsPipeline = SphPipelines;
pub type PhysicsNode = SphPhysicsNode;
