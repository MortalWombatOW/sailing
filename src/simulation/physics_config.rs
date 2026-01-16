//! Physics configuration with per-material-pair interaction parameters.
//!
//! This module defines the canonical physics parameters for the simulation.
//! Instead of hardcoded constants scattered across shaders, all interaction
//! parameters are defined here and sent to the GPU as a uniform buffer.

use bytemuck::{Pod, Zeroable};

/// Material type indices for interaction table lookup.
/// Must match the order used in shaders.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialType {
    Water = 0,
    Air = 1,
    Hull = 2,
    Sail = 3,
    Mast = 4,
}

impl MaterialType {
    /// Number of material types (for table sizing)
    pub const COUNT: usize = 5;

    /// Convert layer_mask to MaterialType
    pub fn from_layer_mask(mask: u32) -> Self {
        if (mask & 1) != 0 { MaterialType::Water }
        else if (mask & 2) != 0 { MaterialType::Air }
        else if (mask & 4) != 0 { MaterialType::Hull }
        else if (mask & 8) != 0 { MaterialType::Sail }
        else { MaterialType::Mast }
    }
}

/// Repulsion ramp type constants (u32 for GPU compatibility)
pub mod repulsion_ramp {
    /// F = k * (1 - r/r0) — immediate stiff resistance
    pub const LINEAR: u32 = 0;
    /// F = k * (1 - r/r0)² — smooth, weaker at edge
    pub const QUADRATIC: u32 = 1;
}

/// Interaction profile between two material types.
/// Defines how particles of type A repel particles of type B.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InteractionProfile {
    /// Repulsion force magnitude (higher = stronger push)
    pub repulsion_strength: f32,
    /// Distance threshold for repulsion (force applied when r < radius)
    pub repulsion_radius: f32,
    /// Ramp type: 0=Linear, 1=Quadratic
    pub repulsion_ramp: u32,
    /// Padding for 16-byte alignment
    pub _padding: u32,
}

impl Default for InteractionProfile {
    fn default() -> Self {
        Self {
            repulsion_strength: 0.0,
            repulsion_radius: 0.0,
            repulsion_ramp: repulsion_ramp::QUADRATIC,
            _padding: 0,
        }
    }
}

impl InteractionProfile {
    /// Create a new interaction profile
    pub fn new(strength: f32, radius: f32, ramp: u32) -> Self {
        Self {
            repulsion_strength: strength,
            repulsion_radius: radius,
            repulsion_ramp: ramp,
            _padding: 0,
        }
    }

    /// No interaction between these material types
    pub fn none() -> Self {
        Self::default()
    }
}

/// Full interaction table for all material pairs.
/// 
/// Indexed by: `profiles[type_a * 5 + type_b]`
/// where type indices are: Water=0, Air=1, Hull=2, Sail=3, Mast=4
/// 
/// Size: 25 profiles × 16 bytes + 16 bytes globals = 416 bytes
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InteractionTable {
    /// 5×5 matrix of interaction profiles
    pub profiles: [InteractionProfile; 25],

    // === Global SPH Parameters ===
    
    /// Viscosity coefficient (fluid thickness)
    pub sph_viscosity: f32,
    /// Pressure stiffness (Tait EOS B parameter)
    pub sph_pressure_stiffness: f32,
    /// Close-range repulsion strength (tensile correction)
    pub sph_close_repulsion: f32,
    /// XSPH velocity smoothing epsilon
    pub xsph_epsilon: f32,
    /// Velocity damping per frame (0-1)
    pub velocity_damping: f32,
    /// Pressure cap to prevent explosions
    pub pressure_cap: f32,
    /// Padding for 16-byte alignment (total: 416 + 24 = 440, pad to 448)
    pub _padding: [f32; 2],
}

impl InteractionTable {
    /// Get the profile index for a material pair
    pub fn index(a: MaterialType, b: MaterialType) -> usize {
        (a as usize) * MaterialType::COUNT + (b as usize)
    }

    /// Set the interaction profile for a material pair (symmetric)
    pub fn set(&mut self, a: MaterialType, b: MaterialType, profile: InteractionProfile) {
        let idx_ab = Self::index(a, b);
        let idx_ba = Self::index(b, a);
        self.profiles[idx_ab] = profile;
        self.profiles[idx_ba] = profile;
    }

    /// Get the interaction profile for a material pair
    pub fn get(&self, a: MaterialType, b: MaterialType) -> &InteractionProfile {
        &self.profiles[Self::index(a, b)]
    }
}

/// The canonical physics configuration for the simulation.
/// 
/// All values are derived from the previously hardcoded constants in forces.wgsl.
/// To tune physics, modify this function and run `cargo test` to verify all
/// scenarios still pass.
pub fn default_interaction_table() -> InteractionTable {
    use MaterialType::*;
    use repulsion_ramp::{LINEAR, QUADRATIC};

    let mut table = InteractionTable {
        profiles: [InteractionProfile::none(); 25],
        
        // Global SPH parameters (from forces.wgsl constants)
        sph_viscosity: 2.0,           // VISCOSITY_MU
        sph_pressure_stiffness: 100.0, // PRESSURE_STIFFNESS
        sph_close_repulsion: 50.0,     // CLOSE_REPULSION
        xsph_epsilon: 0.5,             // XSPH_EPSILON
        velocity_damping: 0.999,       // VELOCITY_DAMPING
        pressure_cap: 2000.0,          // PRESSURE_CAP
        _padding: [0.0; 2],
    };

    // ==================== MATERIAL PAIR INTERACTIONS ====================
    // 
    // From forces.wgsl lines 236-253, fluid↔solid repulsion depends on:
    // - Water↔Hull: LJ_STRENGTH_WATER (100k), LJ_RADIUS_WATER (12.0), Quadratic
    // - Air↔Hull: LJ_STRENGTH_AIR (2M), LJ_RADIUS_AIR (20.0), Linear  
    // - Air↔Sail: Hardcoded 25000 strength, 15.0 radius, Quadratic
    //
    // Same-type pairs (Water↔Water, Air↔Air, etc.) use SPH pressure, not repulsion.
    // ====================================================================

    // Water ↔ Hull: Soft buffer (prevents penetration, water settles stably)
    table.set(Water, Hull, InteractionProfile::new(
        100_000.0,  // LJ_STRENGTH_WATER
        12.0,       // LJ_RADIUS_WATER
        QUADRATIC,
    ));

    // Air ↔ Hull: Stiff wall (no tunneling of high-speed wind)
    table.set(Air, Hull, InteractionProfile::new(
        2_000_000.0,  // LJ_STRENGTH_AIR
        20.0,         // LJ_RADIUS_AIR
        LINEAR,
    ));

    // Air ↔ Sail: Aerodynamic (wind deflects off sail)
    table.set(Air, Sail, InteractionProfile::new(
        25_000.0,  // from forces.wgsl line 244
        15.0,      // from forces.wgsl line 245
        QUADRATIC,
    ));

    // Water ↔ Sail: Minimal interaction (sail is above water at z=2)
    // In practice, z-height filtering prevents this, but define for completeness
    table.set(Water, Sail, InteractionProfile::new(
        50_000.0,
        10.0,
        QUADRATIC,
    ));

    // Mast interactions: Mast is at z=1, isolated (bonds only, no SPH peers)
    // Define minimal repulsion in case particles somehow interact
    table.set(Water, Mast, InteractionProfile::none());
    table.set(Air, Mast, InteractionProfile::none());
    table.set(Hull, Mast, InteractionProfile::none());
    table.set(Sail, Mast, InteractionProfile::none());

    // Same-type interactions use SPH pressure, not direct repulsion
    // (Handled by SPH kernel in shader, not interaction table)
    table.set(Water, Water, InteractionProfile::none());
    table.set(Air, Air, InteractionProfile::none());
    table.set(Hull, Hull, InteractionProfile::none());
    table.set(Sail, Sail, InteractionProfile::none());
    table.set(Mast, Mast, InteractionProfile::none());

    // Hull ↔ Sail: Shouldn't interact (different z levels)
    table.set(Hull, Sail, InteractionProfile::none());

    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_table_size() {
        // Verify struct sizes for GPU compatibility
        assert_eq!(std::mem::size_of::<InteractionProfile>(), 16);
        // 25 profiles * 16 + 6 floats * 4 + 2 padding * 4 = 400 + 24 + 8 = 432
        // Actually: 25*16 = 400, + 8*4 = 32, total = 432. Needs to be 16-aligned = 432 ✓
        let table_size = std::mem::size_of::<InteractionTable>();
        assert_eq!(table_size % 16, 0, "InteractionTable must be 16-byte aligned");
    }

    #[test]
    fn interaction_table_symmetric() {
        let table = default_interaction_table();
        
        // Water↔Hull should be symmetric
        let wh = table.get(MaterialType::Water, MaterialType::Hull);
        let hw = table.get(MaterialType::Hull, MaterialType::Water);
        assert_eq!(wh.repulsion_strength, hw.repulsion_strength);
    }

    #[test]
    fn material_type_from_layer_mask() {
        assert_eq!(MaterialType::from_layer_mask(1), MaterialType::Water);
        assert_eq!(MaterialType::from_layer_mask(2), MaterialType::Air);
        assert_eq!(MaterialType::from_layer_mask(4), MaterialType::Hull);
        assert_eq!(MaterialType::from_layer_mask(8), MaterialType::Sail);
        assert_eq!(MaterialType::from_layer_mask(16), MaterialType::Mast);
    }
}
