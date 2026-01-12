// Physics Integration Shader
// Updates velocity and position, applies soft boundary repulsion AND Bond Forces
// Phase 3: Added Bond Force application from atomic buffer
// Phase 1: Enhanced with soft boundaries to prevent particle alignment

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

struct SimParams {
    delta_time: f32,
    gravity: f32,
    smoothing_radius: f32,
    target_density_water: f32,
    target_density_air: f32,
    wind_interaction_threshold: f32,
    rudder_angle: f32,
    sheet_extension: f32,
    bounds: vec4<f32>, // min_x, max_x, min_y, max_y
    _padding: vec4<f32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;
@group(0) @binding(2) var<storage, read_write> forces: array<atomic<i32>>; // Atomic accumulators [x, y, x, y...]

const FORCE_SCALER: f32 = 1000.0;


// ==================== BOUNDARY PARAMETERS ====================
const BOUNDARY_STIFFNESS: f32 = 0.0;//500.0;    // Repulsion strength - higher = harder bounce
const BOUNDARY_RANGE: f32 = 4.0;         // Distance at which repulsion starts
const BOUNDARY_MARGIN: f32 = 2.0;         // Hard stop margin (safety fallback)
const MAX_VELOCITY: f32 = 500.0;          // Velocity cap to prevent explosions
// =============================================================

// Soft boundary repulsion force - increases as particle approaches wall
fn boundary_force(distance_to_wall: f32) -> f32 {
    if distance_to_wall >= BOUNDARY_RANGE {
        return 0.0;
    }
    if distance_to_wall <= 0.0 {
        return BOUNDARY_STIFFNESS;
    }
    // Smooth ramp: stronger as particle gets closer
    // Uses inverse relationship: force = k * (1 - d/range)^2
    let t = 1.0 - (distance_to_wall / BOUNDARY_RANGE);
    return BOUNDARY_STIFFNESS * t * t;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);

    if idx >= particle_count {
        return;
    }

    var p = particles[idx];

    // ==================== STATIC PARTICLE CHECK ====================
    // If mass is huge (Infinite mass), treat as static obstacle
    if p.mass > 10000.0 {
        p.vel = vec2<f32>(0.0, 0.0);
        // Do not update position
        particles[idx] = p;
        return;
    }
    // ===============================================================

    // ==================== APPLY BOND FORCES ====================
    // Read accumulated bond forces from atomic buffer and apply to velocity
    // Forces are stored as fixed-point integers scaled by FORCE_SCALER
    let fx_int = atomicLoad(&forces[idx * 2u]);
    let fy_int = atomicLoad(&forces[idx * 2u + 1u]);
    
    // Convert back to float
    let bond_force = vec2<f32>(f32(fx_int), f32(fy_int)) / FORCE_SCALER;
    
    // Apply force to velocity: a = F/m, v += a*dt
    p.vel += (bond_force / p.mass) * params.delta_time;
    
    // Clear the force buffer for next frame (reset to 0)
    atomicStore(&forces[idx * 2u], 0);
    atomicStore(&forces[idx * 2u + 1u], 0);
    // ===========================================================

    // ==================== WIND TUNNEL RECYCLING ====================
    let is_air = (p.layer_mask & 2u) != 0u;
    if is_air {
        // If Air particle leaves the Right Edge
        if p.pos.x > params.bounds.y {
             // Teleport to Left Edge
            p.pos.x = params.bounds.x + 5.0; // Small offset to avoid immediate boundary force
             // Reset Velocity to "Wind Speed" (Simulate continuous flow)
            p.vel = vec2<f32>(150.0, 0.0); 
             
             // Keep Y position relative (Laminar flow)
             // Reset Density/Pressure for stability? (Will be recalculated next frame anyway)
            p.density = params.target_density_air;
            p.pressure = 0.0;
        }
    }
    // ===============================================================

    // Calculate distances to each boundary
    let dist_left = p.pos.x - params.bounds.x;
    let dist_right = params.bounds.y - p.pos.x;
    let dist_bottom = p.pos.y - params.bounds.z;
    let dist_top = params.bounds.w - p.pos.y;
    
    // Apply soft boundary forces (before position integration)
    var boundary_vel = vec2<f32>(0.0, 0.0);
    
    // Left wall pushes right (+x)
    boundary_vel.x += boundary_force(dist_left);
    // Right wall pushes left (-x)
    boundary_vel.x -= boundary_force(dist_right);
    // Bottom wall pushes up (+y)
    boundary_vel.y += boundary_force(dist_bottom);
    // Top wall pushes down (-y)
    boundary_vel.y -= boundary_force(dist_top);
    
    // Apply boundary velocity change
    p.vel += boundary_vel * params.delta_time;
    
    // Position integration (velocity already updated by forces shader)
    p.pos += p.vel * params.delta_time;
    
    // Hard boundary fallback (safety - should rarely trigger)
    let min_x = params.bounds.x + BOUNDARY_MARGIN;
    let max_x = params.bounds.y - BOUNDARY_MARGIN;
    let min_y = params.bounds.z + BOUNDARY_MARGIN;
    let max_y = params.bounds.w - BOUNDARY_MARGIN;

    // if p.pos.x < min_x {
    //     p.pos.x = min_x;
    //     p.vel.x = abs(p.vel.x) * 0.3;
    // }
    // if p.pos.x > max_x {
    //     p.pos.x = max_x;
    //     p.vel.x = -abs(p.vel.x) * 0.3;
    // }
    if p.pos.y < min_y {
        p.pos.y = min_y;
        p.vel.y = abs(p.vel.y) * 0.3;
    }
    if p.pos.y > max_y {
        p.pos.y = max_y;
        p.vel.y = -abs(p.vel.y) * 0.3;
    }
    
    // Clamp velocity to prevent explosions
    let vel_len = length(p.vel);
    if vel_len > MAX_VELOCITY {
        p.vel = normalize(p.vel) * MAX_VELOCITY;
    }

    particles[idx] = p;
}
