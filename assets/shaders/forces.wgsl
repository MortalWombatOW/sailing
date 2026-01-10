// SPH Forces Kernel (Brute Force)
// Computes pressure force and viscosity - O(n²) for correctness

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

// Tait's Equation of State for pressure
fn compute_pressure(density: f32, target_density: f32) -> f32 {
    let B = 100.0;  // Stiffness constant
    let gamma = 7.0;
    let ratio = density / target_density;
    return B * (pow(ratio, gamma) - 1.0);
}

// Spiky kernel gradient for pressure force
fn spiky_gradient(r: vec2<f32>, r_len: f32, h: f32) -> vec2<f32> {
    if r_len >= h || r_len < 0.0001 {
        return vec2<f32>(0.0, 0.0);
    }
    let h6 = h * h * h * h * h * h;
    let coeff = -45.0 / (3.14159265359 * h6);
    let diff = h - r_len;
    return coeff * diff * diff * normalize(r);
}

// Viscosity kernel laplacian
fn viscosity_laplacian(r_len: f32, h: f32) -> f32 {
    if r_len >= h {
        return 0.0;
    }
    let h6 = h * h * h * h * h * h;
    let coeff = 45.0 / (3.14159265359 * h6);
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
    
    // Compute pressure from Tait EOS
    p.pressure = compute_pressure(p.density, target_density);
    
    var pressure_force = vec2<f32>(0.0, 0.0);
    var viscosity_force = vec2<f32>(0.0, 0.0);
    let viscosity_mu = 0.5; // Viscosity coefficient (increased for stability)
    
    // Brute force: check all particles
    for (var j = 0u; j < particle_count; j++) {
        if j == idx {
            continue;
        }
        
        let neighbor = particles[j];
        let r = p.pos - neighbor.pos;
        let r_len = length(r);
        
        if r_len >= h || r_len < 0.0001 {
            continue;
        }
        
        // Pressure force (symmetric formulation) - positive = repulsion
        let pressure_term = (p.pressure + neighbor.pressure) / (2.0 * neighbor.density);
        pressure_force += neighbor.mass * pressure_term * spiky_gradient(r, r_len, h);
        
        // Viscosity force
        let vel_diff = neighbor.vel - p.vel;
        viscosity_force += viscosity_mu * neighbor.mass * (vel_diff / neighbor.density) * viscosity_laplacian(r_len, h);
    }
    
    // Apply forces to velocity
    let total_force = pressure_force + viscosity_force;
    p.vel += (total_force / p.density) * params.delta_time;
    
    // Apply gravity with buoyancy
    if is_water {
        // Water sinks
        p.vel.y += params.gravity * params.delta_time;
    } else {
        // Air rises (buoyancy effect)
        p.vel.y += 3.0 * params.delta_time;
    }
    
    // Apply damping to prevent energy buildup
    p.vel *= 0.99;
    
    particles[idx] = p;
}
