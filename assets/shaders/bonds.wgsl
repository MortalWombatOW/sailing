// Peridynamics Bond Kernel
// Computes spring forces between connected particles

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

struct Bond {
    particle_a: u32,
    particle_b: u32,
    rest_length: f32,
    stiffness: f32,
    breaking_strain: f32,
    bond_type: u32,
    is_active: u32,
    _padding: u32,
}

// Fixed-point conversion factor for atomic force accumulation
// Float force * SCALER = Int force
const FORCE_SCALER: f32 = 1000.0; 

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> bonds: array<Bond>;
@group(0) @binding(2) var<storage, read_write> forces: array<atomic<i32>>; // [x0, y0, x1, y1, ...]

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let bond_count = arrayLength(&bonds);

    if idx >= bond_count {
        return;
    }

    var bond = bonds[idx];

    // Skip broken bonds
    if bond.is_active == 0u {
        return;
    }

    let pA = particles[bond.particle_a];
    let pB = particles[bond.particle_b];

    let diff = pB.pos - pA.pos;
    let dist = length(diff);

    // Prevent singularity
    if dist < 0.001 {
        return;
    }

    // Calculate Strain
    let strain = (dist - bond.rest_length) / bond.rest_length;

    // Check Fracture
    if strain > bond.breaking_strain {
        bond.is_active = 0u;
        bonds[idx] = bond;
        return;
    }

    // Calculate Force Modulus (Hooke's Law with Damping)
    // Spring force: F = k * (dist - rest)
    // Damping force: F_d = -c * v_relative_along_spring

    let vel_diff = pB.vel - pA.vel;
    let force_dir = diff / dist;
    
    // Project relative velocity along spring direction
    let rel_vel_along_spring = dot(vel_diff, force_dir);
    
    // Spring force (limited to prevent extreme values)
    let spring_force_raw = bond.stiffness * (dist - bond.rest_length);
    let spring_force = clamp(spring_force_raw, -2000000.0, 2000000.0);
    
    // Damping force (proportional to relative velocity along spring)
    let damping_coeff = 200.0; // Damping coefficient (was 50)
    let damping_force = damping_coeff * rel_vel_along_spring;
    
    // Total force (spring + damping)
    let total_force = spring_force + damping_force;
    let force_vec = force_dir * total_force;

    // Atomic Accumulation with clamping to prevent overflow
    // Scaled by 1000, so 2M becomes 2 billion (fits in i32)
    let force_clamped = clamp(force_vec, vec2<f32>(-2000000.0), vec2<f32>(2000000.0));
    let fx_int = i32(force_clamped.x * FORCE_SCALER);
    let fy_int = i32(force_clamped.y * FORCE_SCALER);

    // Add to A
    atomicAdd(&forces[bond.particle_a * 2u], fx_int);
    atomicAdd(&forces[bond.particle_a * 2u + 1u], fy_int);

    // Subtract from B (Add negative)
    atomicAdd(&forces[bond.particle_b * 2u], -fx_int);
    atomicAdd(&forces[bond.particle_b * 2u + 1u], -fy_int);
}
