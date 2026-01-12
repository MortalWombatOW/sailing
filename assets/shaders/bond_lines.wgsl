// Bond Line Rendering Shader
// Draws bonds as lines connecting particles

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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;
@group(0) @binding(2) var<storage, read> bonds: array<Bond>;

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let bond = bonds[instance_index];
    
    // Skip inactive bonds (render as degenerate line at origin)
    if bond.is_active == 0u {
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        out.color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return out;
    }

    let pA = particles[bond.particle_a];
    let pB = particles[bond.particle_b];
    
    // vertex_index: 0 = point A, 1 = point B
    var pos: vec2<f32>;
    if vertex_index == 0u {
        pos = pA.pos;
    } else {
        pos = pB.pos;
    }

    let world_pos = vec3<f32>(pos.x, pos.y, 0.1); // Slightly above particles
    out.clip_position = view.clip_from_world * vec4<f32>(world_pos, 1.0);
    
    // Color based on strain (green = relaxed, red = stretched, blue = compressed)
    let current_dist = distance(pA.pos, pB.pos);
    let strain = (current_dist - bond.rest_length) / bond.rest_length;

    var color: vec3<f32>;
    if strain > 0.0 {
        // Stretched: green -> red
        let t = clamp(strain / 0.3, 0.0, 1.0);
        color = mix(vec3<f32>(0.3, 0.8, 0.3), vec3<f32>(1.0, 0.2, 0.2), t);
    } else {
        // Compressed: green -> blue
        let t = clamp(-strain / 0.3, 0.0, 1.0);
        color = mix(vec3<f32>(0.3, 0.8, 0.3), vec3<f32>(0.2, 0.4, 1.0), t);
    }

    out.color = vec4<f32>(color, 0.8);

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
