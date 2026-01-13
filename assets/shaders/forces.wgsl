// SPH Forces Kernel (Grid-Based)
// Computes pressure force and viscosity using grid-accelerated neighbor search

// ==================== TUNABLE PARAMETERS ====================
// Pressure (Tait EOS)
const PRESSURE_STIFFNESS: f32 = 100.0;    // B - higher = stronger repulsion (was 5.0)
const PRESSURE_GAMMA: f32 = 7.0;          // Exponent - higher = stiffer at high density  
const PRESSURE_CAP: f32 = 2000.0;         // Maximum pressure to prevent explosions

// Kernel distances
const MIN_DISTANCE: f32 = 0.003;           // Minimum distance to prevent singularity
const CLOSE_REPULSION: f32 = 50.0;         // Enabled - push close particles apart
const CLOSE_RANGE: f32 = 0.3;             // Fraction of h where close repulsion kicks in (0-1)

// Viscosity
const VISCOSITY_MU: f32 = 2.0;            // Fluid thickness - higher = thicker/slower (was 0.5)

// Damping
const VELOCITY_DAMPING: f32 = 0.999;        // Velocity retained per frame (0-1)

// XSPH Smoothing (reduces oscillation)
const XSPH_EPSILON: f32 = 0.5;             // Velocity smoothing strength (0-1, higher = more smoothing)

// Soft-Sphere Hull-Water Repulsion (prevents water penetration)
// Uses bounded linear spring: F = k * (r0 - r) when r < r0
const LJ_STRENGTH_AIR: f32 = 2000000.0;       // Stiff barrier for high-speed air
const LJ_RADIUS_AIR: f32 = 20.0;            // Wide radius to catch fast particles

const LJ_STRENGTH_WATER: f32 = 100000.0;     // Softer barrier for resting water
const LJ_RADIUS_WATER: f32 = 12.0;          // Narrower radius close to hull surface
// =============================================================

struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    mass: f32,
    density: f32,
    pressure: f32,
    z_height: f32,
    layer_mask: u32,
    cell_id: u32,
    _padding: vec2<f32>,
}

struct GridParams {
    cell_size: f32,
    grid_width: u32,
    grid_height: u32,
    grid_origin_x: f32,
    grid_origin_y: f32,
    _padding: vec3<f32>,
}

struct SimParams {
    delta_time: f32,
    gravity: f32,
    smoothing_radius: f32,
    target_density_water: f32,
    target_density_air: f32,
    wind_interaction_threshold: f32,
    rudder_angle: f32,
    sheet_extension: f32,
    bounds: vec4<f32>,
    _padding: vec4<f32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<storage, read> cell_offsets: array<u32>;  // From counting sort: offset[i] = start of cell i
@group(0) @binding(3) var<uniform> grid: GridParams;
@group(0) @binding(4) var<uniform> params: SimParams;

// Tait's Equation of State for pressure
fn compute_pressure(density: f32, target_density: f32) -> f32 {
    let ratio = density / target_density;
    return PRESSURE_STIFFNESS * (pow(ratio, PRESSURE_GAMMA) - 1.0);
}

// Poly6 kernel for XSPH smoothing (2D version)
fn poly6_kernel(r_sq: f32, h: f32) -> f32 {
    let h_sq = h * h;
    if r_sq >= h_sq {
        return 0.0;
    }
    let diff = h_sq - r_sq;
    // 2D poly6: 4 / (π * h^8) * (h² - r²)³
    let h8 = h_sq * h_sq * h_sq * h_sq;
    let coeff = 4.0 / (3.14159265359 * h8);
    return coeff * diff * diff * diff;
}

fn wendland_c2_gradient(r: vec2<f32>, r_len: f32, h: f32) -> vec2<f32> {
    if r_len >= h || r_len < 0.01 {
        return vec2<f32>(0.0, 0.0);
    }

    let q = r_len / h;
    
    // 2D Wendland C2: W(q) = 7/(πh²) * (1-q)⁴ * (1+4q)
    // Gradient: ∇W = -140q/(πh³) * (1-q)³ * r̂
    // Simplified: coeff * (1-q)³ * q * r̂

    let one_minus_q = 1.0 - q;
    let term3 = one_minus_q * one_minus_q * one_minus_q;

    let h3 = h * h * h;
    let coeff = -140.0 / (3.14159265359 * h3);
    
    // r̂ = r / r_len (unit vector)
    return coeff * term3 * q * normalize(r);
}

// Viscosity kernel laplacian (2D version)
fn viscosity_laplacian(r_len: f32, h: f32) -> f32 {
    if r_len >= h {
        return 0.0;
    }
    // 2D viscosity laplacian: 40 / (π * h^5) * (h - r)
    let h5 = h * h * h * h * h;
    let coeff = 40.0 / (3.14159265359 * h5);
    return coeff * (h - r_len);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);

    if idx >= particle_count {
        return;
    }

    var p = particles[idx];
    let h = params.smoothing_radius;
    
    // Determine target density based on particle type
    let is_water = (p.layer_mask & 1u) != 0u;
    let target_density = select(params.target_density_air, params.target_density_water, is_water);
    
    // Compute pressure from Tait EOS with caps
    p.pressure = clamp(compute_pressure(p.density, target_density), 0.0, PRESSURE_CAP);
    
    // Use stored cell_id to derive cell coordinates (avoids floating point precision mismatches)
    let cell_x = i32(p.cell_id % grid.grid_width);
    let cell_y = i32(p.cell_id / grid.grid_width);

    var pressure_force = vec2<f32>(0.0, 0.0);
    var viscosity_force = vec2<f32>(0.0, 0.0);
    var xsph_correction = vec2<f32>(0.0, 0.0);  // XSPH velocity smoothing
    var water_density_around: f32 = 0.0;        // For Archimedes buoyancy on air
    
    // Iterate over 3x3 neighborhood of cells
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let nx = cell_x + dx;
            let ny = cell_y + dy;

            if nx < 0 || ny < 0 || u32(nx) >= grid.grid_width || u32(ny) >= grid.grid_height {
                continue;
            }

            let neighbor_cell_id = u32(ny) * grid.grid_width + u32(nx);
            
            // Counting sort layout: cell_offsets[i] = start, cell_offsets[i+1] = end
            let cell_start = cell_offsets[neighbor_cell_id];
            let cell_end = cell_offsets[neighbor_cell_id + 1u];

            if cell_start >= cell_end {
                continue;
            }

            for (var j = cell_start; j < cell_end; j++) {
                let neighbor_idx = indices[j];
                
                // Skip self
                if neighbor_idx == idx {
                    continue;
                }

                let neighbor = particles[neighbor_idx];
                let r = p.pos - neighbor.pos;
                let r_len = length(r);

                if r_len >= h || r_len < MIN_DISTANCE {
                    continue;
                }
                
                // =============================================================
                // 2.5D LAYER LOGIC (Z-Height Based)
                // z=0: Water, Hull (interact with each other)
                // z=1: Air, Sail (interact with each other)
                // Particles at different z levels do NOT interact!
                // =============================================================
                let layer_water = 1u;
                let layer_air = 2u;
                let layer_hull = 4u;
                let layer_sail = 8u;

                let is_air = (p.layer_mask & layer_air) != 0u;
                let is_water = (p.layer_mask & layer_water) != 0u;
                let neighbor_is_water = (neighbor.layer_mask & layer_water) != 0u;
                let neighbor_is_air = (neighbor.layer_mask & layer_air) != 0u;
                let neighbor_is_hull = (neighbor.layer_mask & layer_hull) != 0u;
                let is_hull = (p.layer_mask & layer_hull) != 0u;
                let is_sail = (p.layer_mask & layer_sail) != 0u;
                let neighbor_is_sail = (neighbor.layer_mask & layer_sail) != 0u;
                let same_layer = (p.layer_mask == neighbor.layer_mask);
                
                // Z-HEIGHT CHECK: Skip interactions between particles at different height levels
                // Using a threshold of 0.5 to allow for small variations
                let z_diff = abs(p.z_height - neighbor.z_height);
                let same_z_level = z_diff < 0.5;

                if !same_z_level {
                    // Different Z levels - no interaction at all!
                    continue;
                }

                // From here on, we know particles are at the same z level
                // z=0: Water ↔ Hull
                // z=1: Mast (isolated - no SPH peers, bonds only)
                // z=2: Air ↔ Sail

                // Standard SPH: Only apply forces between same-type particles
                // OR between fluid and solid at same z (water↔hull, air↔sail)
                let is_fluid = is_water || is_air;
                let is_solid = is_hull || is_sail;
                let neighbor_is_fluid = neighbor_is_water || neighbor_is_air;
                let neighbor_is_solid = neighbor_is_hull || neighbor_is_sail;
                
                // Skip if different fluid types (shouldn't happen at same z anyway)
                if !same_layer && is_fluid && neighbor_is_fluid {
                    continue;
                }
                
                // ==================== SOFT-SPHERE SOLID-FLUID REPULSION ====================
                // Prevents fluid from penetrating solids at the same z level
                if is_fluid != neighbor_is_fluid {
                    // At z=0: Water↔Hull repulsion (strong barrier)
                    // At z=2: Air↔Sail repulsion (sail deflects wind)
                    let is_air_layer = is_air || neighbor_is_air;
                    
                    // Air-sail repulsion - strong enough to deflect wind
                    let strength = select(LJ_STRENGTH_WATER, 25000.0, is_air_layer);
                    let radius = select(LJ_RADIUS_WATER, 15.0, is_air_layer);

                    if r_len < radius {
                        let t = 1.0 - (r_len / radius);
                        // Quadratic ramp for smooth repulsion
                        let force = strength * t * t;
                        pressure_force += force * normalize(r);
                    }
                }
                // =============================================================================
                
                // For fluid↔solid pairs, ONLY use soft-sphere repulsion (skip SPH forces)
                // This prevents double force application and energy gain
                if is_fluid != neighbor_is_fluid {
                    continue;
                }

                // Pressure force (symmetric formulation) - repulsion
                // Only applies to same-type pairs (water↔water, air↔air, hull↔hull, sail↔sail)
                let pressure_term = (p.pressure + neighbor.pressure) / (2.0 * neighbor.density);
                pressure_force -= neighbor.mass * pressure_term * wendland_c2_gradient(r, r_len, h);
                
                // Close-range repulsion to prevent clumping (tensile correction)
                let close_threshold = h * CLOSE_RANGE;
                if r_len < close_threshold {
                    let close_factor = (close_threshold - r_len) / close_threshold;  // 0 at threshold, 1 at 0
                    pressure_force += CLOSE_REPULSION * close_factor * close_factor * normalize(r);
                }
                
                // Viscosity force
                let vel_diff = neighbor.vel - p.vel;
                viscosity_force += VISCOSITY_MU * neighbor.mass * (vel_diff / neighbor.density) * viscosity_laplacian(r_len, h);
                
                // XSPH velocity smoothing - averages velocity with neighbors OF THE SAME TYPE
                if same_layer {
                    let avg_density = (p.density + neighbor.density) * 0.5;
                    let w = poly6_kernel(r_len * r_len, h);
                    xsph_correction += (neighbor.mass / avg_density) * vel_diff * w;
                }
            }
        }
    }
    
    // Apply forces to velocity
    let total_force = pressure_force + viscosity_force;
    p.vel += (total_force / p.density) * params.delta_time;
    
    // Apply XSPH smoothing to reduce oscillation
    p.vel += XSPH_EPSILON * xsph_correction;
    
    // ==================== SAIL AERODYNAMICS ====================
    let is_sail = (p.layer_mask & 8u) != 0u;
    if is_sail {
        // Wind velocity (matching wind tunnel speed)
        let wind_vel = vec2<f32>(150.0, 0.0);
        
        // Force from relative wind (sail catches wind)
        let rel_wind = wind_vel - p.vel;
        let wind_force = 0.15 * rel_wind;  // Sail absorbs some wind momentum
        p.vel += wind_force * params.delta_time;
        
        // Quadratic drag (air resistance on the sail)
        let drag_coeff = 0.3;
        let vel_mag = length(p.vel);
        if vel_mag > 0.1 {
            let drag_force = -drag_coeff * vel_mag * p.vel;
            p.vel += drag_force * params.delta_time;
        }
    }
    // ==========================================================
    
    // Apply damping to prevent energy buildup
    p.vel *= VELOCITY_DAMPING;

    particles[idx] = p;
}
