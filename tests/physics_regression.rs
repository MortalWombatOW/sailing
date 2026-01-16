//! Physics Regression Tests
//!
//! These tests verify that physics parameters produce stable, non-explosive behavior
//! across all scenarios. The "no explosion" invariant is universal.
//!
//! # Running tests
//! ```bash
//! cargo test physics_regression
//! ```

use sailing::simulation::{default_interaction_table, InteractionTable, MaterialType};

// ==================== UNIVERSAL CONSTANTS ====================

/// Maximum velocity threshold - anything above this is an "explosion"
const EXPLOSION_VELOCITY_THRESHOLD: f32 = 500.0;

/// Maximum allowed particle velocity after simulation stabilizes
const STABLE_VELOCITY_THRESHOLD: f32 = 200.0;

// ==================== HELPER FUNCTIONS ====================

/// Assert that no explosion would occur based on interaction table parameters.
/// This is a static analysis check - verifies parameters are in sane ranges.
fn assert_interaction_table_stable(table: &InteractionTable) {
    // Global parameters must be positive and bounded
    assert!(table.sph_viscosity > 0.0, "Viscosity must be positive");
    assert!(table.sph_viscosity < 100.0, "Viscosity too high - will cause instability");
    
    assert!(table.sph_pressure_stiffness > 0.0, "Pressure stiffness must be positive");
    assert!(table.sph_pressure_stiffness < 10000.0, "Pressure stiffness too high");
    
    assert!(table.velocity_damping > 0.9, "Damping too low - energy will accumulate");
    assert!(table.velocity_damping <= 1.0, "Damping must be <= 1.0");
    
    assert!(table.pressure_cap > 0.0, "Pressure cap must be positive");
    assert!(table.xsph_epsilon >= 0.0 && table.xsph_epsilon <= 1.0, "XSPH epsilon must be 0-1");
    
    // Check all material pair profiles
    for i in 0..25 {
        let profile = &table.profiles[i];
        
        // Repulsion strength must be non-negative
        assert!(profile.repulsion_strength >= 0.0, 
            "Profile {} has negative repulsion strength", i);
        
        // If strength > 0, radius must also be > 0
        if profile.repulsion_strength > 0.0 {
            assert!(profile.repulsion_radius > 0.0,
                "Profile {} has strength but no radius", i);
        }
        
        // Repulsion ramp must be valid (0=Linear, 1=Quadratic)
        assert!(profile.repulsion_ramp <= 1,
            "Profile {} has invalid ramp type {}", i, profile.repulsion_ramp);
    }
}

/// Get human-readable name for material pair index
fn profile_name(idx: usize) -> &'static str {
    const NAMES: [&str; 25] = [
        "Water↔Water", "Water↔Air", "Water↔Hull", "Water↔Sail", "Water↔Mast",
        "Air↔Water", "Air↔Air", "Air↔Hull", "Air↔Sail", "Air↔Mast",
        "Hull↔Water", "Hull↔Air", "Hull↔Hull", "Hull↔Sail", "Hull↔Mast",
        "Sail↔Water", "Sail↔Air", "Sail↔Hull", "Sail↔Sail", "Sail↔Mast",
        "Mast↔Water", "Mast↔Air", "Mast↔Hull", "Mast↔Sail", "Mast↔Mast",
    ];
    NAMES[idx]
}

// ==================== TESTS ====================

#[test]
fn test_default_interaction_table_stability() {
    let table = default_interaction_table();
    assert_interaction_table_stable(&table);
}

#[test]
fn test_no_explosion_water_hull_repulsion() {
    let table = default_interaction_table();
    
    // Water↔Hull profile (index 2 or 10)
    let water_hull_idx = MaterialType::Water as usize * 5 + MaterialType::Hull as usize;
    let profile = &table.profiles[water_hull_idx];
    
    // Water-hull repulsion should be quadratic (smoother)
    assert_eq!(profile.repulsion_ramp, 1, "Water↔Hull should use quadratic ramp");
    
    // Strength should be bounded to prevent explosions
    assert!(profile.repulsion_strength <= 500_000.0,
        "Water↔Hull repulsion {} is too strong (max 500k)", profile.repulsion_strength);
}

#[test]
fn test_no_explosion_air_hull_repulsion() {
    let table = default_interaction_table();
    
    // Air↔Hull profile
    let air_hull_idx = MaterialType::Air as usize * 5 + MaterialType::Hull as usize;
    let profile = &table.profiles[air_hull_idx];
    
    // Air-hull can be linear (stiffer) for fast particles
    // But strength must still be bounded
    assert!(profile.repulsion_strength <= 5_000_000.0,
        "Air↔Hull repulsion {} is too strong (max 5M)", profile.repulsion_strength);
    
    // Radius should be wide enough to catch fast particles
    assert!(profile.repulsion_radius >= 10.0,
        "Air↔Hull radius {} is too small", profile.repulsion_radius);
}

#[test]
fn test_no_explosion_air_sail_repulsion() {
    let table = default_interaction_table();
    
    // Air↔Sail profile
    let air_sail_idx = MaterialType::Air as usize * 5 + MaterialType::Sail as usize;
    let profile = &table.profiles[air_sail_idx];
    
    // Air-sail should be gentler than air-hull (sail needs to deflect, not bounce hard)
    assert!(profile.repulsion_strength <= 100_000.0,
        "Air↔Sail repulsion {} is too strong (max 100k)", profile.repulsion_strength);
    
    // Should use quadratic for smooth deflection
    assert_eq!(profile.repulsion_ramp, 1, "Air↔Sail should use quadratic ramp");
}

#[test]
fn test_symmetric_interactions() {
    let table = default_interaction_table();
    
    // All interactions should be symmetric (A↔B == B↔A)
    for a in 0..5 {
        for b in 0..5 {
            let idx_ab = a * 5 + b;
            let idx_ba = b * 5 + a;
            
            let ab = &table.profiles[idx_ab];
            let ba = &table.profiles[idx_ba];
            
            assert_eq!(ab.repulsion_strength, ba.repulsion_strength,
                "{} != {} strength", profile_name(idx_ab), profile_name(idx_ba));
            assert_eq!(ab.repulsion_radius, ba.repulsion_radius,
                "{} != {} radius", profile_name(idx_ab), profile_name(idx_ba));
            assert_eq!(ab.repulsion_ramp, ba.repulsion_ramp,
                "{} != {} ramp", profile_name(idx_ab), profile_name(idx_ba));
        }
    }
}

#[test]
fn test_same_type_no_direct_repulsion() {
    let table = default_interaction_table();
    
    // Same-type particles should NOT have direct repulsion
    // (they use SPH pressure instead)
    for t in 0..5 {
        let idx = t * 5 + t;
        let profile = &table.profiles[idx];
        
        assert_eq!(profile.repulsion_strength, 0.0,
            "{} should have zero direct repulsion (uses SPH instead)", 
            profile_name(idx));
    }
}

#[test]
fn test_global_sph_parameters_bounded() {
    let table = default_interaction_table();
    
    // These are the "tunable knobs" - verify they're in safe ranges
    println!("SPH Parameters:");
    println!("  viscosity: {}", table.sph_viscosity);
    println!("  pressure_stiffness: {}", table.sph_pressure_stiffness);
    println!("  close_repulsion: {}", table.sph_close_repulsion);
    println!("  xsph_epsilon: {}", table.xsph_epsilon);
    println!("  velocity_damping: {}", table.velocity_damping);
    println!("  pressure_cap: {}", table.pressure_cap);
    
    // Damping near 1.0 prevents energy accumulation
    assert!(table.velocity_damping >= 0.99, 
        "Damping {} too low - risk of energy explosion", table.velocity_damping);
    
    // Pressure cap prevents extreme forces
    assert!(table.pressure_cap <= 10000.0,
        "Pressure cap {} too high", table.pressure_cap);
}
