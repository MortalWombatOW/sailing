// Cell ID calculation shader
// Assigns each particle to a grid cell for neighbor searching

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
    cell_size: f32,       // Size of each grid cell (typically 2 * smoothing_radius)
    grid_width: u32,      // Number of cells in X
    grid_height: u32,     // Number of cells in Y
    grid_origin_x: f32,   // World X of grid origin (left edge)
    grid_origin_y: f32,   // World Y of grid origin (bottom edge)
    _padding: vec3<f32>,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> grid: GridParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let particle_count = arrayLength(&particles);
    
    if idx >= particle_count {
        return;
    }
    
    var p = particles[idx];
    
    // Calculate grid cell coordinates
    let local_x = p.pos.x - grid.grid_origin_x;
    let local_y = p.pos.y - grid.grid_origin_y;
    
    // Clamp to valid grid range
    let cell_x = clamp(u32(local_x / grid.cell_size), 0u, grid.grid_width - 1u);
    let cell_y = clamp(u32(local_y / grid.cell_size), 0u, grid.grid_height - 1u);
    
    // Pack into single cell_id (row-major order)
    p.cell_id = cell_y * grid.grid_width + cell_x;
    
    particles[idx] = p;
}
