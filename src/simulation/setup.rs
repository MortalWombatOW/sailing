//! Buffer initialization for the particle simulation.

use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};
use rand::Rng;

use crate::resources::{Bond, GridParams, Particle, SimParams};

// ==================== SIMULATION CONFIG ====================
/// Number of particles in the simulation
pub const PARTICLE_COUNT: usize = 2000;
/// Max number of bonds
pub const BOND_COUNT: usize = 20_000;

// Hull Configuration
const HULL_WIDTH: usize = 40;      // Grid width in particles
const HULL_HEIGHT: usize = 10;     // Grid height in particles
const HULL_SPACING: f32 = 5.0;     // Distance between particles (was 10)
const HULL_START_Y: f32 = 100.0;   // Starting Y position
const HULL_MASS: f32 = 8000.0;      // Mass per hull particle

// Bond Configuration
const BOND_STIFFNESS: f32 = 30_000.0;  // Spring constant (higher = stiffer)
const BOND_BREAKING_STRAIN: f32 = 2.0;    // Break at 200% stretch

// Water Configuration
const WATER_SPAWN_X_MIN: f32 = -600.0;
const WATER_SPAWN_X_MAX: f32 = 600.0;
const WATER_SPAWN_Y_MIN: f32 = -340.0;
const WATER_SPAWN_Y_MAX: f32 = 340.0;
const WATER_FLOW_VX_MIN: f32 = 20.0;  // Ocean current velocity range
const WATER_FLOW_VX_MAX: f32 = 50.0;
const WATER_FLOW_VY_MIN: f32 = -10.0;
const WATER_FLOW_VY_MAX: f32 = 10.0;
// =============================================================

/// Resource holding the particle storage buffer handle
#[derive(Resource)]
pub struct ParticleBuffer(pub Buffer);

impl FromWorld for ParticleBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let mut rng = rand::thread_rng();

        // "Dry Dock" Scenario:
        // 1. Hull Grid (Rigid Body)
        // 2. Water Pool (Buoyancy test)
        
        let mut particles = Vec::with_capacity(PARTICLE_COUNT);
        
        // Spawn Hull Grid
        let start_x = -(HULL_WIDTH as f32 * HULL_SPACING) / 2.0;
        
        for y in 0..HULL_HEIGHT {
            for x in 0..HULL_WIDTH {
                let px = start_x + (x as f32) * HULL_SPACING;
                let py = HULL_START_Y + (y as f32) * HULL_SPACING;
                let mut p = Particle::new_water([px, py], [0.0, 0.0]);
                p.layer_mask = 4; // Hull
                p.mass = HULL_MASS;
                particles.push(p);
            }
        }
        
        let _hull_particle_count = particles.len();
        
        // Calculate hull bounding box (with margin) to exclude water spawning inside
        let hull_min_x = start_x - HULL_SPACING * 2.0;
        let hull_max_x = start_x + (HULL_WIDTH as f32) * HULL_SPACING + HULL_SPACING * 2.0;
        let hull_min_y = HULL_START_Y - HULL_SPACING * 2.0;
        let hull_max_y = HULL_START_Y + (HULL_HEIGHT as f32) * HULL_SPACING + HULL_SPACING * 2.0;
        
        // Fill rest with Water (Top-down view - spread across whole screen)
        // Exclude the hull bounding box to prevent water spawning inside
        while particles.len() < PARTICLE_COUNT {
            let x = rng.gen_range(WATER_SPAWN_X_MIN..WATER_SPAWN_X_MAX);
            let y = rng.gen_range(WATER_SPAWN_Y_MIN..WATER_SPAWN_Y_MAX);
            
            // Skip if inside hull bounding box
            if x > hull_min_x && x < hull_max_x && y > hull_min_y && y < hull_max_y {
                continue;
            }
            
            let vx = rng.gen_range(WATER_FLOW_VX_MIN..WATER_FLOW_VX_MAX);
            let vy = rng.gen_range(WATER_FLOW_VY_MIN..WATER_FLOW_VY_MAX);
            particles.push(Particle::new_water([x, y], [vx, vy]));
        }

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
