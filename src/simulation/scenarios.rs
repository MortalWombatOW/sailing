//! Test Scenarios for the Sailing SPH Simulation
//!
//! To switch scenarios, comment/uncomment the desired scenario in `spawn_particles()`.

use rand::Rng;
use crate::resources::Particle;

// ==================== SCENARIO SELECTION ====================
// Uncomment ONE scenario function call in spawn_particles() below.
// Each scenario returns (particles, hull_bounds) for exclusion zones.
// =============================================================

/// Hull bounding box for exclusion zones (min_x, max_x, min_y, max_y)
pub type HullBounds = Option<(f32, f32, f32, f32)>;

/// Main entry point - calls the selected scenario
pub fn spawn_particles(particle_count: usize) -> (Vec<Particle>, HullBounds) {
    // ========================================
    // SELECT YOUR SCENARIO HERE:
    // ========================================
    
    // scenario_dry_dock(particle_count)          // Hull + Water (default)
    // scenario_water_only(particle_count)     // Just water particles
    // scenario_pressure_washer(particle_count) // Wind hitting a wall
    scenario_hurricane(particle_count)
}

// ==================== SCENARIO CONFIGS ====================

pub mod config {
    // Hull Configuration
    pub const HULL_WIDTH: usize = 40;
    pub const HULL_HEIGHT: usize = 10;
    pub const HULL_SPACING: f32 = 5.0;
    pub const HULL_START_Y: f32 = 100.0;
    pub const HULL_MASS: f32 = 8000.0;
    
    // Water Configuration
    pub const WATER_SPAWN_X_MIN: f32 = -600.0;
    pub const WATER_SPAWN_X_MAX: f32 = 600.0;
    pub const WATER_SPAWN_Y_MIN: f32 = -340.0;
    pub const WATER_SPAWN_Y_MAX: f32 = 340.0;
    pub const WATER_FLOW_VX_MIN: f32 = 20.0;
    pub const WATER_FLOW_VX_MAX: f32 = 50.0;
    pub const WATER_FLOW_VY_MIN: f32 = -10.0;
    pub const WATER_FLOW_VY_MAX: f32 = 10.0;
}

// ==================== SCENARIOS ====================

/// Scenario: Dry Dock
/// Hull grid floating in water. Tests buoyancy and rigid body behavior.
pub fn scenario_dry_dock(particle_count: usize) -> (Vec<Particle>, HullBounds) {
    use config::*;
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(particle_count);
    
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
    
    // Calculate hull bounding box (with margin)
    let hull_bounds = Some((
        start_x - HULL_SPACING * 2.0,
        start_x + (HULL_WIDTH as f32) * HULL_SPACING + HULL_SPACING * 2.0,
        HULL_START_Y - HULL_SPACING * 2.0,
        HULL_START_Y + (HULL_HEIGHT as f32) * HULL_SPACING + HULL_SPACING * 2.0,
    ));
    
    // Fill rest with Water (excluding hull area)
    while particles.len() < particle_count {
        let x = rng.gen_range(WATER_SPAWN_X_MIN..WATER_SPAWN_X_MAX);
        let y = rng.gen_range(WATER_SPAWN_Y_MIN..WATER_SPAWN_Y_MAX);
        
        // Skip if inside hull bounding box
        if let Some((min_x, max_x, min_y, max_y)) = hull_bounds {
            if x > min_x && x < max_x && y > min_y && y < max_y {
                continue;
            }
        }
        
        let vx = rng.gen_range(WATER_FLOW_VX_MIN..WATER_FLOW_VX_MAX);
        let vy = rng.gen_range(WATER_FLOW_VY_MIN..WATER_FLOW_VY_MAX);
        particles.push(Particle::new_water([x, y], [vx, vy]));
    }
    
    (particles, hull_bounds)
}

/// Scenario: Water Only
/// Pure water simulation with no hull. Good for tuning SPH parameters.
#[allow(dead_code)]
pub fn scenario_water_only(particle_count: usize) -> (Vec<Particle>, HullBounds) {
    use config::*;
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(particle_count);
    
    while particles.len() < particle_count {
        let x = rng.gen_range(WATER_SPAWN_X_MIN..WATER_SPAWN_X_MAX);
        let y = rng.gen_range(WATER_SPAWN_Y_MIN..WATER_SPAWN_Y_MAX);
        let vx = rng.gen_range(WATER_FLOW_VX_MIN..WATER_FLOW_VX_MAX);
        let vy = rng.gen_range(WATER_FLOW_VY_MIN..WATER_FLOW_VY_MAX);
        particles.push(Particle::new_water([x, y], [vx, vy]));
    }
    
    (particles, None)
}

// Scenario "Lava Lamp" removed - Top-down simulation has no gravity separation.

/// Scenario: Pressure Washer
/// Wind (air) blasting against a wall of static particles.
#[allow(dead_code)]
pub fn scenario_pressure_washer(particle_count: usize) -> (Vec<Particle>, HullBounds) {
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(particle_count);
    
    // Create vertical wall of static hull particles
    let wall_x = 100.0;
    let wall_height = 60; // 3x more particles (was 20)
    let wall_spacing = 5.0; // 3x denser (was 15.0)
    
    for i in 0..wall_height {
        let y = -((wall_height as f32) * wall_spacing / 2.0) + (i as f32) * wall_spacing;
        let mut p = Particle::new_water([wall_x, y], [0.0, 0.0]);
        p.layer_mask = 4; // Hull (static)
        p.mass = 100000.0; // Very heavy (effectively static)
        particles.push(p);
    }
    
    // Fill rest with Air particles (wind) coming from the left
    while particles.len() < particle_count {
        let x = rng.gen_range(-600.0..-200.0); // Left side
        let y = rng.gen_range(-200.0..200.0);
        let mut p = Particle::new_water([x, y], [150.0, 0.0]); // Wind velocity
        p.layer_mask = 2; // Air
        p.mass = 1.0;
        particles.push(p);
    }
    
    (particles, None)
}

/// Scenario: Hurricane (Top-Down View)
/// High-speed wind test for sail billow and mast fracture verification.
/// Hull, mast, and sail OVERLAP in x,y but have different z_heights.
#[allow(dead_code)]
pub fn scenario_hurricane(particle_count: usize) -> (Vec<Particle>, HullBounds) {
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(particle_count);
    
    // ==================== HURRICANE SCENARIO CONFIG ====================
    // Top-down view: z_height determines interaction layer
    // z=0: Water, Hull (interact with each other)
    // z=1: Mast (isolated - only interacts via bonds)
    // z=2: Sail, Air (interact with each other)
    const HULL_Z: f32 = 0.0;        // Hull at water level (z=0)
    const MAST_Z: f32 = 1.0;        // Mast isolated (z=1)
    const SAIL_Z: f32 = 2.0;        // Sail at air level (z=2)
    
    const SAIL_WIDTH: usize = 2;    // Sail thickness (perpendicular to wind in X)
    const SAIL_HEIGHT: usize = 10;   // Sail length (vertical line in Y direction)  
    const SAIL_SPACING: f32 = 8.0;  // Sail particle spacing
    const WIND_SPEED: f32 = 50.0;  // Gentle wind
    // ===================================================================
    
    // 1. Spawn Hull (rectangular deck - anchored/locked)
    // The hull is a grid at z=0 (water level)
    let hull_start_x = -80.0;
    let hull_start_y = -20.0;
    let hull_width = 20usize;
    let hull_height = 5usize;
    let hull_spacing = 8.0;
    
    for y in 0..hull_height {
        for x in 0..hull_width {
            let px = hull_start_x + (x as f32) * hull_spacing;
            let py = hull_start_y + (y as f32) * hull_spacing;
            let mut p = Particle::new_water([px, py], [0.0, 0.0]);
            p.layer_mask = crate::resources::layer::HULL;
            p.mass = 100000.0; // Effectively static (locked)
            p.z_height = HULL_Z;
            particles.push(p);
        }
    }
    let hull_particle_count = particles.len();
    println!("  Hull particles: 0..{}", hull_particle_count);
    
    // 2. Spawn Mast (single column of particles at center of hull, at high z)
    // In top-down view, mast is a single point but we make it a small cluster
    let mast_x = hull_start_x + (hull_width as f32 * hull_spacing) / 2.0;
    let mast_y = hull_start_y + (hull_height as f32 * hull_spacing) / 2.0;
    let mast_start_idx = particles.len();
    
    // Just a few mast particles clustered at the mast position
    // (In top-down, a vertical mast appears as a small cross-section)
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let px = mast_x + (dx as f32) * 4.0;
            let py = mast_y + (dy as f32) * 4.0;
            let mut p = Particle::new_mast([px, py], [0.0, 0.0]);
            p.z_height = MAST_Z;
            particles.push(p);
        }
    }
    let mast_end_idx = particles.len();
    println!("  Mast particles: {}..{}", mast_start_idx, mast_end_idx);
    
    // 3. Spawn Spar (boom extending from mast along sail length)
    // The spar is a 10x2 grid of mast particles that the sail attaches to
    let spar_start_x = mast_x + SAIL_SPACING; // One spacing unit right of mast
    let spar_start_y = mast_y - (SAIL_HEIGHT as f32 * SAIL_SPACING) / 2.0;
    let spar_start_idx = particles.len();
    const SPAR_DEPTH: usize = 2; // 2 particles deep in X direction
    
    for y in 0..SAIL_HEIGHT {
        for x in 0..SPAR_DEPTH {
            let px = spar_start_x + (x as f32) * SAIL_SPACING;
            let py = spar_start_y + (y as f32) * SAIL_SPACING;
            let mut p = Particle::new_mast([px, py], [0.0, 0.0]); // Same as mast (heavy wood)
            p.z_height = MAST_Z; // Spar at mast level (z=1) - doesn't interact with air
            particles.push(p);
        }
    }
    let spar_end_idx = particles.len();
    println!("  Spar particles: {}..{} ({}x{})", spar_start_idx, spar_end_idx, SPAR_DEPTH, SAIL_HEIGHT);
    
    // 4. Spawn Sail Grid (attached to spar right edge, extends rightward)
    let sail_start_x = spar_start_x + (SPAR_DEPTH as f32) * SAIL_SPACING;  // Just right of spar
    let sail_start_y = spar_start_y;
    let sail_start_idx = particles.len();
    
    for y in 0..SAIL_HEIGHT {
        for x in 0..SAIL_WIDTH {
            let px = sail_start_x + (x as f32) * SAIL_SPACING;
            let py = sail_start_y + (y as f32) * SAIL_SPACING;
            let mut p = Particle::new_sail([px, py], [0.0, 0.0]);
            p.z_height = SAIL_Z;
            particles.push(p);
        }
    }
    let sail_end_idx = particles.len();
    println!("  Sail particles: {}..{} ({}x{})", sail_start_idx, sail_end_idx, SAIL_WIDTH, SAIL_HEIGHT);
    
    // 4. Fill rest with high-speed Air (Hurricane wind from left)
    while particles.len() < particle_count {
        let x = rng.gen_range(-600.0..-200.0); // Left side of screen
        let y = rng.gen_range(-200.0..200.0);
        let p = Particle::new_air([x, y], [WIND_SPEED, 0.0]);
        particles.push(p);
    }
    
    // Return bounds (for water exclusion if needed)
    let hull_bounds = Some((
        hull_start_x - hull_spacing,
        hull_start_x + (hull_width as f32) * hull_spacing + (SAIL_WIDTH as f32) * SAIL_SPACING + hull_spacing,
        hull_start_y - hull_spacing,
        hull_start_y + (hull_height as f32) * hull_spacing + hull_spacing,
    ));
    
    (particles, hull_bounds)
}

/// Hurricane scenario configuration (exported for bond generation)
/// NOTE: Must match values in scenario_hurricane()!
pub mod hurricane_config {
    // Hull: 20x5 = 100 particles
    pub const HULL_WIDTH: usize = 20;
    pub const HULL_HEIGHT: usize = 5;
    pub const HULL_SPACING: f32 = 8.0;
    
    // Mast: 3x3 = 9 particles (cross-section in top-down view)
    pub const MAST_GRID_SIZE: usize = 3;
    pub const MAST_SPACING: f32 = 4.0;
    
    // Spar: Grid of particles extending from mast (same length as sail, 2 deep)
    pub const SPAR_LENGTH: usize = 10; // Same as SAIL_HEIGHT
    pub const SPAR_DEPTH: usize = 2;   // 2 particles deep in X direction
    
    // Sail: 2x10 = 20 particles (vertical line perpendicular to wind)
    pub const SAIL_WIDTH: usize = 2;
    pub const SAIL_HEIGHT: usize = 10;
    pub const SAIL_SPACING: f32 = 8.0;
}


