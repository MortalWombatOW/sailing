//! Particle rendering module - custom render pipeline for particle instancing.

use bevy::{
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        component::Component,
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    prelude::*,
    render::{
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        sync_world::MainEntity,
        view::{ExtractedView, ViewUniformOffset, ViewUniforms},
        Render, RenderApp, RenderSet,
    },
};

use crate::simulation::{ParticleBuffer, PARTICLE_COUNT};

/// Plugin for rendering particles as instanced dots.
pub struct ParticleRenderPlugin;

impl Plugin for ParticleRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_command::<Transparent2d, DrawParticles>()
            .add_systems(Render, prepare_pipeline.in_set(RenderSet::Prepare))
            .add_systems(Render, queue_particles.in_set(RenderSet::Queue));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ParticleRenderPipeline>();
        // Spawn the particle entity in render world once
        render_app.world_mut().spawn(ExtractedParticles);
    }
}

/// Marker component for extracted particle rendering
#[derive(Component)]
pub struct ExtractedParticles;

/// Particle render pipeline resource
#[derive(Resource)]
pub struct ParticleRenderPipeline {
    pipeline: CachedRenderPipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for ParticleRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Bind group layout for view uniforms
        let bind_group_layout = render_device.create_bind_group_layout(
            Some("Particle View Bind Group Layout"),
            &[
                // View uniform
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Particle buffer
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/particles.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Particle Render Pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 4,
                ..default()
            },
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// Bind group for particle rendering
#[derive(Resource)]
pub struct ParticleBindGroup(pub BindGroup);

/// Marker to track if we should render this frame
#[derive(Resource)]
pub struct ParticleRenderReady;

/// Prepare the particle render bind group
pub fn prepare_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<ParticleRenderPipeline>,
    particle_buffer: Option<Res<ParticleBuffer>>,
    view_uniforms: Res<ViewUniforms>,
) {
    // Remove old ready marker
    commands.remove_resource::<ParticleRenderReady>();

    let Some(particle_buffer) = particle_buffer else {
        return;
    };

    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    let bind_group = render_device.create_bind_group(
        Some("Particle Render Bind Group"),
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: view_binding,
            },
            BindGroupEntry {
                binding: 1,
                resource: particle_buffer.0.as_entire_binding(),
            },
        ],
    );

    commands.insert_resource(ParticleBindGroup(bind_group));
    commands.insert_resource(ParticleRenderReady);
}

/// Queue particles for rendering
pub fn queue_particles(
    mut transparent_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    particle_query: Query<Entity, With<ExtractedParticles>>,
    views: Query<Entity, With<ExtractedView>>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<ParticleRenderPipeline>,
    ready: Option<Res<ParticleRenderReady>>,
) {
    // Only queue if we are ready
    if ready.is_none() {
        return;
    }

    let Ok(particle_entity) = particle_query.get_single() else {
        return;
    };

    let draw_particles = draw_functions.read().id::<DrawParticles>();

    for view_entity in &views {
        let Some(transparent_phase) = transparent_phases.get_mut(&view_entity) else {
            continue;
        };

        transparent_phase.add(Transparent2d {
            sort_key: bevy::math::FloatOrd(0.0),
            entity: (particle_entity, MainEntity::from(particle_entity)),
            pipeline: pipeline.pipeline,
            draw_function: draw_particles,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::NONE,
        });
    }
}

/// Render command for drawing particles
pub type DrawParticles = (SetItemPipeline, SetParticleBindGroup, DrawParticleInstances);

/// Set the particle bind group
pub struct SetParticleBindGroup;

impl<P: PhaseItem> RenderCommand<P> for SetParticleBindGroup {
    type Param = SRes<ParticleBindGroup>;
    type ViewQuery = &'static ViewUniformOffset;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform_offset: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(0, &bind_group.into_inner().0, &[view_uniform_offset.offset]);
        RenderCommandResult::Success
    }
}

/// Draw particle instances
pub struct DrawParticleInstances;

impl<P: PhaseItem> RenderCommand<P> for DrawParticleInstances {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // Draw 6 vertices (2 triangles for quad) per particle instance
        pass.draw(0..6, 0..PARTICLE_COUNT as u32);
        RenderCommandResult::Success
    }
}
