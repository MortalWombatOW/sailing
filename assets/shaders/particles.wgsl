// Particle vertex/fragment shader - renders particles as instanced quads
// Phase 0: Basic particle rendering with color based on velocity

struct View {
    clip_from_world: mat4x4<f32>,
    unjittered_clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    view_from_clip: mat4x4<f32>,
    world_position: vec3<f32>,
    exposure: f32,
    viewport: vec4<f32>,
    frustum: array<vec4<f32>, 6>,
    color_grading: mat3x3<f32>,
    mip_bias: f32,
}

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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;

// Quad vertices (2 triangles)
const QUAD_VERTICES: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, 1.0),
);

const PARTICLE_SIZE: f32 = 3.0;

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let particle = particles[instance_index];
    let quad_vertex = QUAD_VERTICES[vertex_index];
    
    // Calculate world position
    let world_pos = vec3<f32>(
        particle.pos.x + quad_vertex.x * PARTICLE_SIZE,
        particle.pos.y + quad_vertex.y * PARTICLE_SIZE,
        0.0
    );
    
    // Transform to clip space
    out.clip_position = view.clip_from_world * vec4<f32>(world_pos, 1.0);
    
    // UV for circular shape
    out.uv = quad_vertex * 0.5 + 0.5;
    
    // Color based on particle type and velocity
    let speed = length(particle.vel);
    let normalized_speed = clamp(speed / 100.0, 0.0, 1.0);
    
    // Check particle type by layer_mask
    let is_water = (particle.layer_mask & 1u) != 0u;
    let is_air = (particle.layer_mask & 2u) != 0u;
    let is_hull = (particle.layer_mask & 4u) != 0u;

    var color: vec3<f32>;
    if is_hull {
        // Hull particles: brown/wood color
        let base_color = vec3<f32>(0.55, 0.35, 0.2);
        let stressed_color = vec3<f32>(0.7, 0.4, 0.2); // Lighter when moving fast
        color = mix(base_color, stressed_color, normalized_speed);
    } else if is_water {
        // Water particles: blue color range
        let base_color = vec3<f32>(0.1, 0.3, 0.7);
        let fast_color = vec3<f32>(0.3, 0.6, 1.0);
        color = mix(base_color, fast_color, normalized_speed);
    } else {
        // Air particles: white/light gray color range
        let base_color = vec3<f32>(0.8, 0.85, 0.9);
        let fast_color = vec3<f32>(1.0, 1.0, 1.0);
        color = mix(base_color, fast_color, normalized_speed);
    }

    out.color = vec4<f32>(color, 1.0);

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create circular particle shape
    let center = in.uv - vec2<f32>(0.5);
    let dist = length(center);
    
    // Smooth edge
    let alpha = 1.0 - smoothstep(0.4, 0.5, dist);

    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(in.color.rgb, alpha);
}
