// Physics compute shader - simple integration: pos += vel * dt
// Phase 0: Basic particle movement with boundary reflection

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
    _padding: vec4<f32>,  // Padding for 16-byte alignment (64 bytes total)
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);
    
    if idx >= particle_count {
        return;
    }
    
    var p = particles[idx];
    
    // Simple integration: pos += vel * dt
    p.pos += p.vel * params.delta_time;
    
    // Boundary reflection
    let min_x = params.bounds.x;
    let max_x = params.bounds.y;
    let min_y = params.bounds.z;
    let max_y = params.bounds.w;
    
    // Reflect off boundaries
    if p.pos.x < min_x {
        p.pos.x = min_x;
        p.vel.x = abs(p.vel.x);
    } else if p.pos.x > max_x {
        p.pos.x = max_x;
        p.vel.x = -abs(p.vel.x);
    }
    
    if p.pos.y < min_y {
        p.pos.y = min_y;
        p.vel.y = abs(p.vel.y);
    } else if p.pos.y > max_y {
        p.pos.y = max_y;
        p.vel.y = -abs(p.vel.y);
    }
    
    particles[idx] = p;
}
