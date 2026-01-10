//! Buffer initialization for the particle simulation.

use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};
use rand::Rng;

use crate::resources::{Particle, SimParams};

/// Number of particles in the simulation
pub const PARTICLE_COUNT: usize = 10_000;

/// Resource holding the particle storage buffer handle
#[derive(Resource)]
pub struct ParticleBuffer(pub Buffer);

impl FromWorld for ParticleBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let mut rng = rand::thread_rng();

        // Initialize particles with random positions and velocities
        let particles: Vec<Particle> = (0..PARTICLE_COUNT)
            .map(|_| {
                let pos = [
                    rng.gen_range(-600.0..600.0),
                    rng.gen_range(-340.0..340.0),
                ];
                let vel = [
                    rng.gen_range(-50.0..50.0),
                    rng.gen_range(-50.0..50.0),
                ];
                Particle::new_water(pos, vel)
            })
            .collect();

        // Create particle storage buffer
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&particles),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::VERTEX,
        });

        Self(buffer)
    }
}

/// Resource holding the simulation parameters uniform buffer handle
#[derive(Resource)]
pub struct SimParamsBuffer(pub Buffer);

impl FromWorld for SimParamsBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create simulation params uniform buffer
        let sim_params = SimParams::default();
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("SimParams Buffer"),
            contents: bytemuck::bytes_of(&sim_params),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self(buffer)
    }
}
