// Prefix Sum Shader (Pass 2 of Counting Sort)
// Computes exclusive prefix sum of cell counts -> cell offsets
// Uses simple sequential approach (runs on single workgroup for small cell counts)

struct GridParams {
    cell_size: f32,
    grid_width: u32,
    grid_height: u32,
    grid_origin_x: f32,
    grid_origin_y: f32,
    _padding: vec3<f32>,
}

@group(0) @binding(0) var<storage, read> cell_counts: array<u32>;
@group(0) @binding(1) var<storage, read_write> cell_offsets: array<u32>;
@group(0) @binding(2) var<uniform> grid: GridParams;

// Simple sequential prefix sum (fine for ~2300 cells)
// Only thread 0 does the work - inefficient but correct
@compute @workgroup_size(1)
fn prefix_sum(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let total_cells = grid.grid_width * grid.grid_height;
    
    var running_sum = 0u;
    
    for (var i = 0u; i < total_cells; i++) {
        // Exclusive prefix sum: offset[i] = sum of counts[0..i)
        cell_offsets[i] = running_sum;
        running_sum += cell_counts[i];
    }
    
    // Store total at the end (for bounds checking)
    cell_offsets[total_cells] = running_sum;
}
