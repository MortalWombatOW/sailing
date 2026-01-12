//! Buffer initialization for the particle simulation.

use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};

use crate::resources::{Bond, GridParams, SimParams};
use super::scenarios;

// ==================== SIMULATION CONFIG ====================
/// Number of particles in the simulation
pub const PARTICLE_COUNT: usize = 8000;
/// Max number of bonds
pub const BOND_COUNT: usize = 20_000;

// Bond Configuration (used by BondBuffer)
const BOND_STIFFNESS: f32 = 30_000.0;
const BOND_BREAKING_STRAIN: f32 = 2.0;
// =============================================================

/// Resource holding the particle storage buffer handle
#[derive(Resource)]
pub struct ParticleBuffer(pub Buffer);

impl FromWorld for ParticleBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Use the scenarios module to spawn particles
        let (particles, _hull_bounds) = scenarios::spawn_particles(PARTICLE_COUNT);

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

/// Resource holding the bond buffer (Storage)
#[derive(Resource)]
pub struct BondBuffer(pub Buffer);

impl FromWorld for BondBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        
        use scenarios::config::*;
        
        let mut bonds = Vec::new();
        let diagonal_length = HULL_SPACING * 1.4142; // sqrt(2) for diagonal bonds
        
        for y in 0..HULL_HEIGHT {
            for x in 0..HULL_WIDTH {
                let idx = y * HULL_WIDTH + x;
                
                // Horizontal bond (right neighbor)
                if x + 1 < HULL_WIDTH {
                    let right_idx = y * HULL_WIDTH + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: right_idx as u32,
                        rest_length: HULL_SPACING,
                        stiffness: BOND_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Vertical bond (top neighbor)
                if y + 1 < HULL_HEIGHT {
                    let top_idx = (y + 1) * HULL_WIDTH + x;
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: top_idx as u32,
                        rest_length: HULL_SPACING,
                        stiffness: BOND_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Diagonal bond (top-right)
                if x + 1 < HULL_WIDTH && y + 1 < HULL_HEIGHT {
                    let tr_idx = (y + 1) * HULL_WIDTH + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: tr_idx as u32,
                        rest_length: diagonal_length,
                        stiffness: BOND_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Diagonal bond (top-left)
                if x > 0 && y + 1 < HULL_HEIGHT {
                    let tl_idx = (y + 1) * HULL_WIDTH + (x - 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: tl_idx as u32,
                        rest_length: diagonal_length,
                        stiffness: BOND_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0,
                        is_active: 1,
                        _padding: 0,
                    });
                }
            }
        }
        
        while bonds.len() < BOND_COUNT {
            bonds.push(Bond {
                particle_a: 0,
                particle_b: 0,
                rest_length: 0.0,
                stiffness: 0.0,
                breaking_strain: 0.0,
                bond_type: 0,
                is_active: 0,
                _padding: 0,
            });
        }
        
        println!("Generated {} bonds for hull.", bonds.len());

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Bond Buffer"),
            contents: bytemuck::cast_slice(&bonds),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        Self(buffer)
    }
}

/// Resource holding the atomic force accumulation buffer
#[derive(Resource)]
pub struct ForceBuffer(pub Buffer);

impl FromWorld for ForceBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        
        let size = PARTICLE_COUNT * 2 * 4;
        
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("Atomic Force Buffer"),
            size: size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        Self(buffer)
    }
}
