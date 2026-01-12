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
    
    scenario_dry_dock(particle_count)          // Hull + Water (default)
    // scenario_water_only(particle_count)     // Just water particles
    // scenario_pressure_washer(particle_count) // Wind hitting a wall
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
