# Work Log

## 2026-01-10: Phase 0 - GPU Boilerplate Complete

### Summary
Implemented the foundational GPU compute and render pipeline for particle simulation using Bevy 0.15.

### Key Learnings for Future Phases

#### 1. Bevy 0.15 API Changes
- **`entry_point`**: Use `"name".into()` (Cow), not `Some("name".into())` (Option)
- **`FloatOrd`**: Import from `bevy::math::FloatOrd`, not `bevy::sprite`
- **Transparent2d entity**: Requires `(Entity, MainEntity)` tuple, not just `Entity`
- **RenderCommand ViewQuery**: Use `&'static ViewUniformOffset` instead of `Read<...>`

#### 2. Render App vs Main App Resources
- **Critical**: `RenderDevice` only exists in the **render app**, not the main app
- Buffer initialization must happen in `finish()` using `FromWorld` trait, not in a main app `Startup` system
- `Extract<Option<Res<T>>>` reads from **main app** - if your resource only exists in render app, this returns `None`

#### 3. Render Pipeline Compatibility (2D Transparent Pass)
Bevy's 2D transparent render pass has specific requirements:
- **Depth stencil**: `Depth32Float` with `CompareFunction::GreaterEqual`
- **MSAA sample count**: `4` (not the default `1`)
- Without these, you get cryptic GPU validation errors about "incompatible render pass"

#### 4. Render Phase Timing
- Use a "ready marker" resource pattern to gate queuing until bind groups are prepared
- `SRes<T>` in RenderCommand panics if resource doesn't exist - ensure resources are created before queuing render commands

#### 5. WGSL Uniform Buffer Alignment
- Uniform buffers must be 16-byte aligned in total size
- A Rust struct with 14 × f32 = 56 bytes must be padded to 64 bytes (16 × f32)
- Mismatch causes: "Buffer is bound with size X where shader expects Y"

### Files Created
- `Cargo.toml`, `src/main.rs`, `src/resources.rs`
- `src/simulation/{mod.rs, setup.rs, systems.rs}`
- `src/render/mod.rs`
- `assets/shaders/{physics.wgsl, particles.wgsl}`

## 2026-01-10: Phase 1 - Multi-Phase SPH Fluid Complete

### Summary
Implemented WCSPH physics with Air/Water particle separation. Water sinks (blue), Air rises (white).

### Key Learnings for Future Phases

#### 1. Brute Force vs Grid-Based Neighbor Search
- Grid-based sorting (Bitonic Sort + CellStartEnd) is complex to debug
- Started with O(n²) brute force for correctness, then optimize later
- With 10k particles the GPU handles O(n²) fine on RTX 4070 Ti

#### 2. SPH Pressure Force Sign
- **Critical bug**: `pressure_force -= mass * term * gradient` = ATTRACTION!
- Correct: `pressure_force += mass * term * gradient` for REPULSION
- The spiky kernel gradient has a -45 coefficient that flips the direction

#### 3. WGSL vec3 Alignment
- WGSL `vec3<f32>` has 16-byte alignment, not 12!
- A struct with 5 f32s + vec3 padding must be 48 bytes (not 32)
- Rust padding: use `[f32; 7]` instead of `[f32; 3]` for a vec3<f32> field

#### 4. SPH Parameter Tuning
- Hollywood mass ratios: Water=10, Air=1 (not real 1000:1)
- Stiffness B=100 for softer look, higher for incompressible
- Viscosity 0.5 for stability, damping 0.99 to prevent explosions
- Air buoyancy: explicit upward force (+3) instead of negative gravity

### Files Created/Modified
- `assets/shaders/{cell_id.wgsl, sort.wgsl, build_grid.wgsl, density.wgsl, forces.wgsl}`
- `src/simulation/setup.rs` - Added GridParams, IndexBuffer, CellRangeBuffer, SortParamsBuffer
- `src/resources.rs` - Added GridParams struct, Particle::new_air()


## 2026-01-11: SPH Tuning - Vortices & Stability

### Summary
Addressed stability issues in the SPH simulation, specifically persistent vortices, particle alignment at boundaries, and tensile instability (clumping).

### Key Learnings

#### 1. Hard Boundaries vs Soft Repulsion
- **Problem**: Hard clamping (`max(pos.x, min_x)`) causes particles to line up in perfect vertical columns at the walls ("crystalline stacking").
- **Fix**: Replaced hard clamping with a **soft repulsion force** that ramps up as particles approach the boundary (`BOUNDARY_RANGE`). This breaks the alignment and creates natural interaction.

#### 2. Stable Vortices (Convection Cells)
- **Problem**: Simulation settled into stable, rotating vortices (like convection cells) instead of calming down.
- **Root Cause**: Likely a combination of **XSPH velocity smoothing** (which averages neighbor velocities, enforcing coherent rotation) and **high timestep** (adding energy).
- **Fix**: 
    - Disabled **XSPH** (`0.0`) to break velocity coherence.
    - Increased **Viscosity** (`0.5`) to dampen rotation.
    - Reduced **Timestep** (`0.04`) to prevent energy overshoot.

#### 3. Artificial Pressure (Tensile Instability)
- **Problem**: Particles clumping together in pairs or clusters (tensile instability).
- **Attempt**: Added `CLOSE_REPULSION` (Monaghan artificial pressure) to push close particles apart.
- **Finding**: While it prevents clumping, if set too high (`1000.0`), it acts like a spring adding massive energy to the system, causing explosions/oscillations. Disabled it (`0.0`) in favor of correct pressure formulation.

#### 4. Kernel Choice: Wendland vs Poly6
- **Problem**: Poly6 kernel (gradient) is 0 at r=0, allowing particles to stack on top of each other.
- **Fix**: Switched to **Wendland C2** kernel for both density and pressure forces.
- **Correction**: The 2D Wendland gradient formula was initially incorrect (missing `q` factor). Fixed: `∇W = -140q/(πh³) * (1-q)³ * r̂`.

#### 5. Pressure Force Formulation
- **Correction**: The pressure force was using an asymmetric formula `(Pi + Pj)/(2*ρj)`.
- **Fix**: Switched to standard symmetric SPH: `Pi/ρi² + Pj/ρj²`. This ensures forces are equal and opposite (Newton's 3rd), conserving momentum.
