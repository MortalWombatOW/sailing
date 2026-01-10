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
            mass: 1.0,
            density: 0.0,
            pressure: 0.0,
            z_height: 0.0,
            layer_mask: 1, // Water
            cell_id: 0,
            _padding: [0.0; 2],
        }
    }
}

/// Layer mask constants for particle types
pub mod layer {
    pub const WATER: u32 = 1;
    pub const AIR: u32 = 2;
    pub const HULL: u32 = 4;
    pub const SAIL: u32 = 8;
    pub const MAST: u32 = 16;
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
            delta_time: 0.005, // 5ms fixed timestep as per AGENT.md
            gravity: -9.8,
            smoothing_radius: 10.0,
            target_density_water: 1000.0,
            target_density_air: 1.0,
            wind_interaction_threshold: 0.5,
            rudder_angle: 0.0,
            sheet_extension: 1.0,
            bounds: [-640.0, 640.0, -360.0, 360.0], // 1280x720 centered
            _padding: [0.0; 4],
        }
    }
}
