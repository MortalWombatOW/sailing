//! Buffer initialization for the particle simulation.

use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};
use rand::Rng;

use crate::resources::{GridParams, Particle, SimParams};

/// Number of particles in the simulation
pub const PARTICLE_COUNT: usize = 10_000;

/// Resource holding the particle storage buffer handle
#[derive(Resource)]
pub struct ParticleBuffer(pub Buffer);

impl FromWorld for ParticleBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let mut rng = rand::thread_rng();

        // "Lava Lamp" initialization: mixed Air and Water
        // Water in top half (will sink), Air in bottom half (will rise)
        let particles: Vec<Particle> = (0..PARTICLE_COUNT)
            .map(|i| {
                let pos = [
                    rng.gen_range(-600.0..600.0),
                    rng.gen_range(-340.0..340.0),
                ];
                let vel = [
                    rng.gen_range(-5.0..5.0),
                    rng.gen_range(-5.0..5.0),
                ];
                
                // Alternate particles for good mixing
                if i % 2 == 0 {
                    Particle::new_water(pos, vel)
                } else {
                    Particle::new_air(pos, vel)
                }
            })
            .collect();

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::VERTEX,
        });

        Self(buffer)
    }
}

/// Resource holding the simulation parameters uniform buffer handle
#[derive(Resource)]
pub struct SimParamsBuffer(pub Buffer);

impl FromWorld for SimParamsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let sim_params = SimParams::default();
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("SimParams Buffer"),
            contents: bytemuck::bytes_of(&sim_params),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self(buffer)
    }
}

/// Resource holding the grid parameters uniform buffer
#[derive(Resource)]
pub struct GridParamsBuffer(pub Buffer);

impl FromWorld for GridParamsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let grid_params = GridParams::default();
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("GridParams Buffer"),
            contents: bytemuck::bytes_of(&grid_params),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self(buffer)
    }
}

/// Resource holding particle index buffer for sorting
#[derive(Resource)]
pub struct IndexBuffer(pub Buffer);

impl FromWorld for IndexBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Initialize with identity mapping (0, 1, 2, ...)
        let indices: Vec<u32> = (0..PARTICLE_COUNT as u32).collect();
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        Self(buffer)
    }
}

/// Resource holding cell counts buffer for counting sort (Pass 1)
#[derive(Resource)]
pub struct CellCountsBuffer(pub Buffer);

impl FromWorld for CellCountsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let grid_params = GridParams::default();
        let total_cells = (grid_params.grid_width * grid_params.grid_height) as usize;
        
        // One u32 count per cell
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("CellCounts Buffer"),
            size: (total_cells * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self(buffer)
    }
}

/// Resource holding cell offsets buffer for counting sort (Pass 2/3)
/// Size is total_cells + 1 for the end sentinel
#[derive(Resource)]
pub struct CellOffsetsBuffer(pub Buffer);

impl FromWorld for CellOffsetsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let grid_params = GridParams::default();
        let total_cells = (grid_params.grid_width * grid_params.grid_height) as usize;
        
        // One u32 offset per cell + 1 for end sentinel
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("CellOffsets Buffer"),
            size: ((total_cells + 1) * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self(buffer)
    }
}

/// Sort parameters for bitonic sort
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SortParams {
    pub block_size: u32,
    pub sub_block_size: u32,
    pub particle_count: u32,
    pub _padding: u32,
}

/// Resource holding sort parameters uniform buffer
#[derive(Resource)]
pub struct SortParamsBuffer(pub Buffer);

impl FromWorld for SortParamsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let sort_params = SortParams {
            block_size: 2,
            sub_block_size: 2,
            particle_count: PARTICLE_COUNT as u32,
            _padding: 0,
        };
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("SortParams Buffer"),
            contents: bytemuck::bytes_of(&sort_params),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self(buffer)
    }
}
