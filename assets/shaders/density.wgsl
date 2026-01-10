// SPH Density Kernel (Brute Force)
// Computes density for each particle - O(n²) for correctness, will optimize later

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
    bounds: vec4<f32>,
    _padding: vec4<f32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

// Poly6 kernel for density estimation
// W(r, h) = 315 / (64 * π * h^9) * (h² - r²)³
fn poly6_kernel(r_sq: f32, h: f32) -> f32 {
    let h_sq = h * h;
    if r_sq >= h_sq {
        return 0.0;
    }
    let diff = h_sq - r_sq;
    let h9 = h_sq * h_sq * h_sq * h_sq * h;
    let coeff = 315.0 / (64.0 * 3.14159265359 * h9);
    return coeff * diff * diff * diff;
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
    
    var density = 0.0;
    
    // Brute force: check all particles
    for (var j = 0u; j < particle_count; j++) {
        let neighbor = particles[j];
        let diff = p.pos - neighbor.pos;
        let r_sq = dot(diff, diff);
        
        // Accumulate density contribution
        density += neighbor.mass * poly6_kernel(r_sq, h);
    }
    
    // Ensure minimum density to prevent division by zero
    p.density = max(density, 0.001);
    
    particles[idx] = p;
}
