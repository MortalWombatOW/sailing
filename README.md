# Sailing SPH Sim

## 1. Executive Summary

**Project:** A 2.5D top-down sailing simulation.
**Core Mechanic:** "Pure Physics." Every interaction—buoyancy, propulsion, destruction—is the result of particle interactions using Smoothed Particle Hydrodynamics (SPH) and Peridynamics.
**Key Features:**

* Multi-phase fluid (Water + Air) on the GPU.
* Peridynamic solids (Ship Hull, Masts, Sails) capable of fracture/tearing.
* 2.5D Logic (Height-based interaction filtering).
* High-performance GPU Compute (wgpu) + Bevy ECS for state management.

## 2. Technical Stack & Architecture

* **Engine:** Bevy (Latest Stable)
* **Language:** Rust
* **Graphics API:** wgpu (via Bevy)
* **Architecture Pattern:** Host-Device Split
* **Host (CPU/Bevy):** Handles Input, Game State, Asset Loading, Uniform Buffer updates.
* **Device (GPU/Compute):** Handles Physics Integration, Neighbor Search, Force Calculation, Constraint Solving.



### 2.1 The "Hard Path" Constraints

1. **No Fake Physics:** Do not use `bevy_xpbd` or Box2D. All movement must result from SPH particle pressure or Peridynamic bond forces.
2. **No CPU Physics:** The particle count target is 100k+. The CPU cannot touch the particle buffer during the game loop.
3. **Bitonic Sort:** Do not use Hash Maps for neighbor searching. Use a Grid Index Sort (Bitonic Sort) approach to ensure memory coalescing.

---

## 3. Data Structures (The "Bible")

**Constraint:** These structs must be defined in Rust with `#[repr(C)]` and strictly match the WGSL struct definitions.

### 3.1 The Particle

This is the atom of the universe.

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Particle {
    pos: Vec2,         // Position (x, y)
    vel: Vec2,         // Velocity (vx, vy)
    mass: f32,         // Mass constant
    density: f32,      // Calculated by SPH solver
    pressure: f32,     // Calculated by SPH solver
    
    // 2.5D & Logic Flags
    z_height: f32,     // Fake visual height. 0.0=Water Surface.
    layer_mask: u32,   // Bitmask: 1=Water, 2=Air, 4=Hull, 8=Sail, 16=Mast
    
    // Grid Sorting (Internal use)
    cell_id: u32,      // The spatial grid cell this particle is in
    padding: [f32; 1], // Alignment padding to 16 bytes if necessary
}

```

### 3.2 The Bond (Peridynamics)

Connecting solids together.

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Bond {
    particle_a: u32,      // Index of particle A
    particle_b: u32,      // Index of particle B
    rest_length: f32,     // The distance at which force is zero
    stiffness: f32,       // Spring constant (Young's Modulus analog)
    breaking_strain: f32, // If (current_len - rest_len)/rest_len > this, bond dies
    bond_type: u32,       // 0=Hull(Rigid), 1=Sail(Cloth), 2=Sheet(Control), 3=MastStep(Fuse)
    active: u32,          // 1 = Active, 0 = Broken
}

```

### 3.3 Simulation Uniforms

Global settings sent from Bevy to GPU every frame.

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SimParams {
    delta_time: f32,
    gravity: f32,
    smoothing_radius: f32, // "h" in SPH literature
    target_density_water: f32,
    target_density_air: f32,
    
    // 2.5D Rules
    wind_interaction_threshold: f32, // Height of wave required to block wind
    
    // Controls
    rudder_angle: f32,
    sheet_extension: f32, // Multiplier for Sail bond rest lengths
}

```