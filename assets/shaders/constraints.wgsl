// Position-Based Collision Constraints
// Prevents particles from penetrating too deeply by direct position correction

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
@group(0) @binding(2) var<storage, read> cell_offsets: array<u32>;
@group(0) @binding(3) var<uniform> grid: GridParams;
@group(0) @binding(4) var<uniform> params: SimParams;

// Minimum distance fraction of smoothing radius
const PBD_RADIUS_FACTOR: f32 = 0.5; 
// How much of the overlap to correct per step (0.0 - 1.0)
// Lower values are more stable but "squishier". 
const STIFFNESS: f32 = 0.8; 

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);

    if idx >= particle_count {
        return;
    }

    var p = particles[idx];
    
    // Skip static particles (infinite mass)
    if p.mass > 10000.0 {
        return;
    }

    let h = params.smoothing_radius;
    let collision_dist = h * PBD_RADIUS_FACTOR;
    let cell_x = i32(p.cell_id % grid.grid_width);
    let cell_y = i32(p.cell_id / grid.grid_width);

    var correction = vec2<f32>(0.0, 0.0);
    var corrections_count = 0.0;

    // Iterate neighbors
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let nx = cell_x + dx;
            let ny = cell_y + dy;

            if nx < 0 || ny < 0 || u32(nx) >= grid.grid_width || u32(ny) >= grid.grid_height {
                continue;
            }

            let neighbor_cell_id = u32(ny) * grid.grid_width + u32(nx);
            let cell_start = cell_offsets[neighbor_cell_id];
            let cell_end = cell_offsets[neighbor_cell_id + 1u];

            for (var j = cell_start; j < cell_end; j++) {
                let neighbor_idx = indices[j];
                if neighbor_idx == idx { continue; }

                let neighbor = particles[neighbor_idx];
                
                // Z-Level Check
                if abs(p.z_height - neighbor.z_height) > 0.5 {
                    continue;
                }

                let diff = p.pos - neighbor.pos;
                let dist = length(diff);

                if dist < 0.001 || dist >= collision_dist {
                    continue;
                }

                // Collision detected!
                let overlap = collision_dist - dist;
                let dir = diff / dist;
                
                // Displace away from neighbor
                // Weight by inverse mass? For now assume equal mass roughly or just geometric
                correction += dir * overlap;
                corrections_count += 1.0;
            }
        }
    }

    if corrections_count > 0.0 {
        // Apply averaged correction
        // We divide by count to avoid over-correcting in dense clusters
        // But maybe we want the SUM to push out of the cluster?
        // Let's try partial correction of the SUM

        let move_vec = (correction / corrections_count) * STIFFNESS;
        // Limit max movement to avoid instability
        // Relaxed limit to handle high-speed wind (6.0 units/frame)
        let max_move = h * 1.0;
        let move_len = length(move_vec);
        if move_len > max_move {
            p.pos += (move_vec / move_len) * max_move;
        } else {
            p.pos += move_vec;
        }

        // Write back
        particles[idx] = p;
    }
}
