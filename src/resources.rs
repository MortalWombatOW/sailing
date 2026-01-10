//! GPU-compatible data structures for the particle simulation.
//!
//! All structs use `#[repr(C)]` and implement `Pod`/`Zeroable` for GPU buffer compatibility.

use bytemuck::{Pod, Zeroable};

/// The fundamental particle in the SPH simulation.
///
/// This struct is the "atom" of our physics universe. All movement results from
/// particle interactions via SPH pressure or Peridynamic bond forces.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Particle {
    /// Position (x, y) in world coordinates
    pub pos: [f32; 2],
    /// Velocity (vx, vy)
    pub vel: [f32; 2],
    /// Mass constant (affects pressure calculations)
    pub mass: f32,
    /// Density calculated by SPH solver
    pub density: f32,
    /// Pressure calculated by SPH solver
    pub pressure: f32,
    /// Fake visual height for 2.5D logic. 0.0 = Water Surface
    pub z_height: f32,
    /// Bitmask: 1=Water, 2=Air, 4=Hull, 8=Sail, 16=Mast
    pub layer_mask: u32,
    /// Spatial grid cell this particle belongs to (for sorting)
    pub cell_id: u32,
    /// Padding for 16-byte alignment (48 bytes total)
    pub _padding: [f32; 2],
}

impl Particle {
    /// Create a new water particle at the given position with velocity
    pub fn new_water(pos: [f32; 2], vel: [f32; 2]) -> Self {
        Self {
            pos,
            vel,
            mass: 10.0, // "Hollywood ratio" - heavier than air
            density: 0.0,
            pressure: 0.0,
            z_height: 0.0,
            layer_mask: layer::WATER,
            cell_id: 0,
            _padding: [0.0; 2],
        }
    }

    /// Create a new air particle at the given position with velocity
    pub fn new_air(pos: [f32; 2], vel: [f32; 2]) -> Self {
        Self {
            pos,
            vel,
            mass: 1.0, // "Hollywood ratio" - lighter than water
            density: 0.0,
            pressure: 0.0,
            z_height: 0.0,
            layer_mask: layer::AIR,
            cell_id: 0,
            _padding: [0.0; 2],
        }
    }
}

/// Layer mask constants for particle types
#[allow(dead_code)]
pub mod layer {
    pub const WATER: u32 = 1;
    pub const AIR: u32 = 2;
    pub const HULL: u32 = 4;
    pub const SAIL: u32 = 8;
    pub const MAST: u32 = 16;
}

/// Grid parameters for spatial indexing (neighbor search)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct GridParams {
    /// Size of each grid cell (typically 2 * smoothing_radius)
    pub cell_size: f32,
    /// Number of cells in X direction
    pub grid_width: u32,
    /// Number of cells in Y direction
    pub grid_height: u32,
    /// World X coordinate of grid origin (left edge)
    pub grid_origin_x: f32,
    /// World Y coordinate of grid origin (bottom edge)
    pub grid_origin_y: f32,
    /// Padding for WGSL vec3 alignment (total 48 bytes)
    /// Note: WGSL vec3<f32> has 16-byte alignment, so we need 7 f32s padding
    pub _padding: [f32; 7],
}

impl Default for GridParams {
    fn default() -> Self {
        let cell_size = 20.0; // 2 * smoothing_radius (10.0)
        let grid_origin_x = -640.0;
        let grid_origin_y = -360.0;
        let world_width = 1280.0;
        let world_height = 720.0;
        
        Self {
            cell_size,
            grid_width: (world_width / cell_size).ceil() as u32,
            grid_height: (world_height / cell_size).ceil() as u32,
            grid_origin_x,
            grid_origin_y,
            _padding: [0.0; 7],
        }
    }
}

/// Global simulation parameters sent to GPU each frame.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct SimParams {
    /// Time step for integration
    pub delta_time: f32,
    /// Gravity strength (negative = downward)
    pub gravity: f32,
    /// SPH smoothing radius (h)
    pub smoothing_radius: f32,
    /// Target density for water particles
    pub target_density_water: f32,
    /// Target density for air particles
    pub target_density_air: f32,
    /// Wave height threshold for wind-water interaction
    pub wind_interaction_threshold: f32,
    /// Rudder angle in radians
    pub rudder_angle: f32,
    /// Sheet extension multiplier for sail bonds
    pub sheet_extension: f32,
    /// Screen bounds for boundary reflection (min_x, max_x, min_y, max_y)
    pub bounds: [f32; 4],
    /// Padding for 16-byte alignment (total size: 64 bytes = 16 f32s)
    pub _padding: [f32; 4],
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            delta_time: 0.032, // 32ms timestep (doubled for faster simulation)
            gravity: -5.0,     // Reduced gravity for less aggressive falling
            smoothing_radius: 10.0,
            target_density_water: 1000.0,
            target_density_air: 100.0, // Increased from 1.0 for better pressure response
            wind_interaction_threshold: 0.5,
            rudder_angle: 0.0,
            sheet_extension: 1.0,
            bounds: [-640.0, 640.0, -360.0, 360.0], // 1280x720 centered
            _padding: [0.0; 4],
        }
    }
}
