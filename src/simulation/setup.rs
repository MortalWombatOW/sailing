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
pub const PARTICLE_COUNT: usize = 10000;
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
        
        // ==================== BOND STIFFNESS CONFIG ====================
        const HULL_STIFFNESS: f32 = 30_000.0;      // Rigid hull
        const SAIL_STIFFNESS: f32 = 15_000.0;      // Rigid sail panel
        const MAST_STIFFNESS: f32 = 20_000.0;      // Stiff mast
        const FUSE_STIFFNESS: f32 = 100_000.0;     // Mast-hull connection (extremely strong)
        const FUSE_BREAKING_STRAIN: f32 = 10.0;    // Essentially unbreakable
        // ===============================================================
        
        use scenarios::config::*;
        use scenarios::hurricane_config;
        
        let mut bonds = Vec::new();
        let diagonal_length = HULL_SPACING * 1.4142; // sqrt(2) for diagonal bonds
        
        // =========================== HULL BONDS ===========================
        // Note: These may be for dry_dock or hurricane scenario
        // For hurricane, hull is just a single row, but we still generate bonds
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
                        stiffness: HULL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0, // Hull
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
                        stiffness: HULL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0, // Hull
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
                        stiffness: HULL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0, // Hull
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
                        stiffness: HULL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0, // Hull
                        is_active: 1,
                        _padding: 0,
                    });
                }
            }
        }
        
        let hull_bond_count = bonds.len();
        
        // =========================== HURRICANE SCENARIO BONDS ===========================
        // These are for the hurricane test scenario with sail and mast (TOP-DOWN VIEW)
        // Indices must match scenario_hurricane() particle ordering:
        //   Hull: 0..100 (20x5 grid)
        //   Mast: 100..109 (3x3 grid cross-section)
        //   Sail: 109..157 (8x6 grid)
        
        let hurricane_hull_count = hurricane_config::HULL_WIDTH * hurricane_config::HULL_HEIGHT; // 100
        let mast_particle_count = hurricane_config::MAST_GRID_SIZE * hurricane_config::MAST_GRID_SIZE; // 9
        let mast_start_idx = hurricane_hull_count;
        let mast_end_idx = mast_start_idx + mast_particle_count;
        let sail_start_idx = mast_end_idx;
        
        // --- MAST BONDS (3x3 grid with horizontal/vertical + diagonal for rigidity) ---
        let mast_size = hurricane_config::MAST_GRID_SIZE;
        for y in 0..mast_size {
            for x in 0..mast_size {
                let idx = mast_start_idx + y * mast_size + x;
                
                // Horizontal bond
                if x + 1 < mast_size {
                    let right_idx = mast_start_idx + y * mast_size + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: right_idx as u32,
                        rest_length: hurricane_config::MAST_SPACING,
                        stiffness: MAST_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0, // Mast (rigid)
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Vertical bond  
                if y + 1 < mast_size {
                    let top_idx = mast_start_idx + (y + 1) * mast_size + x;
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: top_idx as u32,
                        rest_length: hurricane_config::MAST_SPACING,
                        stiffness: MAST_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Diagonal bond (for rigidity)
                if x + 1 < mast_size && y + 1 < mast_size {
                    let diag_idx = mast_start_idx + (y + 1) * mast_size + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: diag_idx as u32,
                        rest_length: hurricane_config::MAST_SPACING * 1.4142,
                        stiffness: MAST_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 0,
                        is_active: 1,
                        _padding: 0,
                    });
                }
            }
        }
        
        // --- MAST-HULL FUSE BONDS (breakable connection) ---
        // Connect center mast particle(s) to hull particles underneath
        // Hull center is at index (HULL_HEIGHT/2 * HULL_WIDTH + HULL_WIDTH/2)
        let hull_w = hurricane_config::HULL_WIDTH;
        let hull_h = hurricane_config::HULL_HEIGHT;
        let hull_center_idx = (hull_h / 2) * hull_w + (hull_w / 2);
        let mast_center_idx = mast_start_idx + (mast_size / 2) * mast_size + (mast_size / 2);
        
        // Connect mast center to hull center (main fuse)
        bonds.push(Bond {
            particle_a: hull_center_idx as u32,
            particle_b: mast_center_idx as u32,
            rest_length: 0.1, // Very short - they are overlapping!
            stiffness: FUSE_STIFFNESS,
            breaking_strain: FUSE_BREAKING_STRAIN,
            bond_type: 3, // MastStep/Fuse
            is_active: 1,
            _padding: 0,
        });
        
        // Connect mast edges to nearby hull particles for stability
        for m_offset in [0, mast_size - 1] {
            for h_offset in [0isize, 1, -1] {
                let mast_idx = mast_start_idx + m_offset;
                let hull_idx = (hull_center_idx as isize + h_offset) as usize;
                if hull_idx < hurricane_hull_count {
                    bonds.push(Bond {
                        particle_a: hull_idx as u32,
                        particle_b: mast_idx as u32,
                        rest_length: hurricane_config::MAST_SPACING,
                        stiffness: FUSE_STIFFNESS,
                        breaking_strain: FUSE_BREAKING_STRAIN,
                        bond_type: 3,
                        is_active: 1,
                        _padding: 0,
                    });
                }
            }
        }
        
        // --- SAIL BONDS (with diagonals for rigidity - holds shape better) ---
        let sail_w = hurricane_config::SAIL_WIDTH;
        let sail_h = hurricane_config::SAIL_HEIGHT;
        let sail_diag_length = hurricane_config::SAIL_SPACING * 1.4142;
        
        for y in 0..sail_h {
            for x in 0..sail_w {
                let idx = sail_start_idx + y * sail_w + x;
                
                // Horizontal bond (right neighbor)
                if x + 1 < sail_w {
                    let right_idx = sail_start_idx + y * sail_w + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: right_idx as u32,
                        rest_length: hurricane_config::SAIL_SPACING,
                        stiffness: SAIL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 1, // Sail
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Vertical bond (top neighbor)
                if y + 1 < sail_h {
                    let top_idx = sail_start_idx + (y + 1) * sail_w + x;
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: top_idx as u32,
                        rest_length: hurricane_config::SAIL_SPACING,
                        stiffness: SAIL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 1, // Sail
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Diagonal bond (top-right) - provides shear resistance
                if x + 1 < sail_w && y + 1 < sail_h {
                    let diag_idx = sail_start_idx + (y + 1) * sail_w + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: diag_idx as u32,
                        rest_length: sail_diag_length,
                        stiffness: SAIL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 1, // Sail
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Diagonal bond (top-left) - provides shear resistance
                if x > 0 && y + 1 < sail_h {
                    let diag_idx = sail_start_idx + (y + 1) * sail_w + (x - 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: diag_idx as u32,
                        rest_length: sail_diag_length,
                        stiffness: SAIL_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 1, // Sail
                        is_active: 1,
                        _padding: 0,
                    });
                }
            }
        }
        
        // --- SPAR BONDS (10x2 grid) ---
        // Spar particles start after mast: mast_start_idx + 9 = spar_start
        let spar_start_idx = mast_start_idx + mast_size * mast_size; // 100 + 9 = 109
        let spar_len = hurricane_config::SPAR_LENGTH;
        let spar_depth = hurricane_config::SPAR_DEPTH;
        let spar_diag = hurricane_config::SAIL_SPACING * 1.4142;
        
        // Spar internal bonds (grid with horizontal, vertical, diagonal)
        for y in 0..spar_len {
            for x in 0..spar_depth {
                let idx = spar_start_idx + y * spar_depth + x;
                
                // Horizontal bond (right neighbor)
                if x + 1 < spar_depth {
                    let right_idx = spar_start_idx + y * spar_depth + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: right_idx as u32,
                        rest_length: hurricane_config::SAIL_SPACING,
                        stiffness: MAST_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 2,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Vertical bond (next y)
                if y + 1 < spar_len {
                    let next_y_idx = spar_start_idx + (y + 1) * spar_depth + x;
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: next_y_idx as u32,
                        rest_length: hurricane_config::SAIL_SPACING,
                        stiffness: MAST_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 2,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Diagonal bonds for rigidity
                if x + 1 < spar_depth && y + 1 < spar_len {
                    let diag_idx = spar_start_idx + (y + 1) * spar_depth + (x + 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: diag_idx as u32,
                        rest_length: spar_diag,
                        stiffness: MAST_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 2,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                if x > 0 && y + 1 < spar_len {
                    let diag_idx = spar_start_idx + (y + 1) * spar_depth + (x - 1);
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: diag_idx as u32,
                        rest_length: spar_diag,
                        stiffness: MAST_STIFFNESS,
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 2,
                        is_active: 1,
                        _padding: 0,
                    });
                }
                
                // Skip-2 vertical bonds for bending resistance (keeps spar straight)
                if y + 2 < spar_len {
                    let skip_idx = spar_start_idx + (y + 2) * spar_depth + x;
                    bonds.push(Bond {
                        particle_a: idx as u32,
                        particle_b: skip_idx as u32,
                        rest_length: hurricane_config::SAIL_SPACING * 2.0,
                        stiffness: MAST_STIFFNESS * 2.0, // Extra stiff for bending resistance
                        breaking_strain: BOND_BREAKING_STRAIN,
                        bond_type: 2,
                        is_active: 1,
                        _padding: 0,
                    });
                }
            }
        }
        
        // Mast-center to Spar left column bonds (connect mast to spar)
        let mast_center = mast_start_idx + (mast_size / 2) * mast_size + (mast_size / 2);
        for y in 0..spar_len {
            let spar_left_idx = spar_start_idx + y * spar_depth + 0; // Left column of spar
            let y_offset = (y as f32 - (spar_len as f32 / 2.0)).abs() * hurricane_config::SAIL_SPACING;
            let rest_len = (hurricane_config::SAIL_SPACING.powi(2) + y_offset.powi(2)).sqrt();
            
            bonds.push(Bond {
                particle_a: mast_center as u32,
                particle_b: spar_left_idx as u32,
                rest_length: rest_len.max(hurricane_config::SAIL_SPACING),
                stiffness: FUSE_STIFFNESS, // Same as mast-hull (extremely strong)
                breaking_strain: BOND_BREAKING_STRAIN,
                bond_type: 2,
                is_active: 1,
                _padding: 0,
            });
        }
        
        // --- SAIL-SPAR BONDS (connect sail left edge to spar right column) ---
        let spar_total = spar_len * spar_depth; // 10 * 2 = 20
        let sail_start_idx_fixed = spar_start_idx + spar_total; // 109 + 20 = 129
        
        for y in 0..sail_h {
            let spar_right_idx = spar_start_idx + y * spar_depth + (spar_depth - 1); // Right column of spar
            let sail_left_idx = sail_start_idx_fixed + y * sail_w; // Left edge of sail
            
            bonds.push(Bond {
                particle_a: spar_right_idx as u32,
                particle_b: sail_left_idx as u32,
                rest_length: hurricane_config::SAIL_SPACING,
                stiffness: SAIL_STIFFNESS * 2.0,
                breaking_strain: BOND_BREAKING_STRAIN,
                bond_type: 1,
                is_active: 1,
                _padding: 0,
            });
        }
        
        let total_active_bonds = bonds.len();
        println!("Generated {} hull bonds + {} sail/mast bonds = {} total", 
                 hull_bond_count, total_active_bonds - hull_bond_count, total_active_bonds);
        
        // Pad to BOND_COUNT
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
