// SPH Density Kernel (Grid-Based)
// Computes density for each particle using grid-accelerated neighbor search

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

fn wendland_c2_kernel(r_sq: f32, h: f32) -> f32 {
    // Wendland C2 has support radius 2h, so q ∈ [0, 2]
    let r = sqrt(r_sq);
    let q = r / h;

    if q >= 2.0 {
        return 0.0;
    }
    
    // 2D Wendland C2: 7 / (4πh^2) * (1 - q/2)^4 * (2q + 1)
    let one_minus_q_half = 1.0 - q * 0.5;
    let term2 = one_minus_q_half * one_minus_q_half;
    let term4 = term2 * term2;

    let h_sq = h * h;
    let coeff = 7.0 / (4.0 * 3.14159265359 * h_sq);

    return coeff * term4 * (2.0 * q + 1.0);
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
    
    // Use stored cell_id to derive cell coordinates (avoids floating point precision mismatches)
    // cell_id = cell_y * grid_width + cell_x, so we reverse it
    let cell_x = i32(p.cell_id % grid.grid_width);
    let cell_y = i32(p.cell_id / grid.grid_width);

    var density = 0.0;
    
    // Iterate over 3x3 neighborhood of cells
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let nx = cell_x + dx;
            let ny = cell_y + dy;
            
            // Skip out-of-bounds cells
            if nx < 0 || ny < 0 || u32(nx) >= grid.grid_width || u32(ny) >= grid.grid_height {
                continue;
            }

            let neighbor_cell_id = u32(ny) * grid.grid_width + u32(nx);
            
            // Counting sort layout: cell_offsets[i] = start, cell_offsets[i+1] = end
            let cell_start = cell_offsets[neighbor_cell_id];
            let cell_end = cell_offsets[neighbor_cell_id + 1u];
            
            // Skip empty cells
            if cell_start >= cell_end {
                continue;
            }
            
            // Iterate over particles in this cell
            for (var j = cell_start; j < cell_end; j++) {
                let neighbor_idx = indices[j];
                let neighbor = particles[neighbor_idx];

                let diff = p.pos - neighbor.pos;
                let r_sq = dot(diff, diff);
                
                // Z-HEIGHT CHECK: Skip density contribution from particles at different z levels
                // z=0: Water, Hull
                // z=1: Air, Sail, Mast
                let z_diff = abs(p.z_height - neighbor.z_height);
                if z_diff > 0.5 {
                    continue;
                }
                
                // Layer Logic:
                // 1. Same layer always interacts
                // 2. Solids (Hull, Sail, Mast) interact with fluids at same z level
                // 3. Water (1) and Air (2) do NOT interact directly (they're at different z anyway)
                let layer_mask_hull = 4u;
                let layer_mask_sail = 8u;
                let layer_mask_mast = 16u;
                let is_hull = (p.layer_mask & layer_mask_hull) != 0u;
                let neighbor_is_hull = (neighbor.layer_mask & layer_mask_hull) != 0u;
                let is_solid = (p.layer_mask & (layer_mask_hull | layer_mask_sail | layer_mask_mast)) != 0u;
                let neighbor_is_solid = (neighbor.layer_mask & (layer_mask_hull | layer_mask_sail | layer_mask_mast)) != 0u;
                let same_layer = (p.layer_mask == neighbor.layer_mask);

                if !same_layer && !is_solid && !neighbor_is_solid {
                    continue;
                }

                // Accumulate density contribution
                density += neighbor.mass * wendland_c2_kernel(r_sq, h);
            }
        }
    }
    
    // Ensure minimum density to prevent division by zero / force explosion
    p.density = max(density, 0.1);

    particles[idx] = p;
}
