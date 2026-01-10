// Build Grid Shader
// Scans sorted particle indices to find cell boundaries (start/end for each cell)

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

// Each cell stores the start and end index in the sorted array
struct CellRange {
    start: u32,
    end: u32,
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
@group(0) @binding(2) var<storage, read_write> cell_ranges: array<CellRange>;
@group(0) @binding(3) var<uniform> grid: GridParams;

// First pass: clear all cells
@compute @workgroup_size(256)
fn clear_cells(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell_idx = global_id.x;
    let total_cells = grid.grid_width * grid.grid_height;
    
    if cell_idx >= total_cells {
        return;
    }
    
    // Set to invalid range (start > end means empty)
    cell_ranges[cell_idx] = CellRange(0xFFFFFFFFu, 0u);
}

// Second pass: find boundaries
@compute @workgroup_size(256)
fn build_grid(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&indices);
    
    if idx >= particle_count {
        return;
    }
    
    let particle_idx = indices[idx];
    let cell_id = particles[particle_idx].cell_id;
    
    // Check if this is the start of a new cell
    if idx == 0u {
        // First particle always starts its cell
        cell_ranges[cell_id].start = idx;
    } else {
        let prev_particle_idx = indices[idx - 1u];
        let prev_cell_id = particles[prev_particle_idx].cell_id;
        
        if cell_id != prev_cell_id {
            // This particle starts a new cell
            cell_ranges[cell_id].start = idx;
            // Previous particle ends its cell
            cell_ranges[prev_cell_id].end = idx;
        }
    }
    
    // Check if this is the last particle
    if idx == particle_count - 1u {
        cell_ranges[cell_id].end = idx + 1u;
    }
}
