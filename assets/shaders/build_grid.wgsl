// Build Grid Shader
// Scans sorted particle indices to find cell boundaries (start/end for each cell)
// Uses atomic operations to prevent race conditions

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

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> cell_ranges: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> grid: GridParams;

// Cell range layout: each cell has 2 u32s - [start, end]
// Index calculation: cell_id * 2 + 0 = start, cell_id * 2 + 1 = end

// First pass: clear all cells
@compute @workgroup_size(256)
fn clear_cells(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell_idx = global_id.x;
    let total_cells = grid.grid_width * grid.grid_height;
    
    if cell_idx >= total_cells {
        return;
    }
    
    // Initialize start to max value (atomicMin will find the minimum)
    // Initialize end to 0 (atomicMax will find the maximum)
    atomicStore(&cell_ranges[cell_idx * 2u], 0xFFFFFFFFu);  // start
    atomicStore(&cell_ranges[cell_idx * 2u + 1u], 0u);      // end
}

// Second pass: find boundaries using atomics
@compute @workgroup_size(256)
fn build_grid(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&indices);
    
    if idx >= particle_count {
        return;
    }
    
    let particle_idx = indices[idx];
    let cell_id = particles[particle_idx].cell_id;
    
    // Thread-safe update: each particle contributes to its cell's range
    // atomicMin ensures start is the smallest index in this cell
    // atomicMax ensures end is the largest index + 1 in this cell
    atomicMin(&cell_ranges[cell_id * 2u], idx);
    atomicMax(&cell_ranges[cell_id * 2u + 1u], idx + 1u);
}
