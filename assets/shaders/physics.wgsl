// Physics Integration Shader
// Updates velocity and position, applies boundaries
// Phase 1: Enhanced with boundary damping

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

const BOUNDARY_DAMPING: f32 = 0.5;
const BOUNDARY_MARGIN: f32 = 5.0;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);
    
    if idx >= particle_count {
        return;
    }
    
    var p = particles[idx];
    
    // Position integration (velocity already updated by forces shader)
    p.pos += p.vel * params.delta_time;
    
    // Boundary handling with damping
    let min_x = params.bounds.x + BOUNDARY_MARGIN;
    let max_x = params.bounds.y - BOUNDARY_MARGIN;
    let min_y = params.bounds.z + BOUNDARY_MARGIN;
    let max_y = params.bounds.w - BOUNDARY_MARGIN;
    
    // Left boundary
    if p.pos.x < min_x {
        p.pos.x = min_x;
        p.vel.x = abs(p.vel.x) * BOUNDARY_DAMPING;
    }
    // Right boundary
    if p.pos.x > max_x {
        p.pos.x = max_x;
        p.vel.x = -abs(p.vel.x) * BOUNDARY_DAMPING;
    }
    // Bottom boundary
    if p.pos.y < min_y {
        p.pos.y = min_y;
        p.vel.y = abs(p.vel.y) * BOUNDARY_DAMPING;
    }
    // Top boundary
    if p.pos.y > max_y {
        p.pos.y = max_y;
        p.vel.y = -abs(p.vel.y) * BOUNDARY_DAMPING;
    }
    
    // Clamp velocity to prevent explosions
    let max_vel = 500.0;
    let vel_len = length(p.vel);
    if vel_len > max_vel {
        p.vel = normalize(p.vel) * max_vel;
    }
    
    particles[idx] = p;
}
