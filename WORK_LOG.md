# Work Log

## 2026-01-13: Phase 4 Continued - Energy Conservation & Bond Strengthening

### Summary
Further refined sail physics with focus on energy conservation, stronger bond connections, and visual improvements.

### Key Fixes

#### 1. Energy Conservation Fix (Air Bouncing with More Speed)
**Problem:** Air particles were gaining speed after bouncing off the sail - violating energy conservation.

**Root Cause:** Fluid↔solid pairs (air↔sail) received BOTH:
- Soft-sphere repulsion (distance-based)
- SPH pressure + close-range repulsion + viscosity

This double force application caused energy gain.

**Solution:** After soft-sphere repulsion, `continue` to skip SPH forces for fluid↔solid pairs.
```wgsl
if is_fluid != neighbor_is_fluid {
    continue;  // Only soft-sphere, no SPH forces
}
```

#### 2. Spar Bending Resistance (Skip-2 Bonds)
**Problem:** Spar bent inward ( shape) when sail pulled on it.

**Solution:** Added skip-2 vertical bonds connecting every other particle (y+2) with 2× stiffness. This creates bending resistance similar to a real beam.

#### 3. Bond Stiffness Updates
| Connection | Old Stiffness | New Stiffness |
|------------|---------------|---------------|
| Fuse (Mast-Hull) | 30,000 | **100,000** |
| Mast-Spar | 20,000 | **100,000** |
| Breaking strain | 2.0 | **10.0** |

#### 4. Air Particle Rendering
- Air particles now render at **half size** (1.5 vs 3.0)
- Less visual clutter, easier to see sail/hull

### Files Changed
- `assets/shaders/forces.wgsl` - Skip SPH for fluid↔solid, increase air-sail repulsion to 25,000
- `assets/shaders/particles.wgsl` - Air particles render at half size
- `src/simulation/setup.rs` - Skip-2 spar bonds, increased fuse/mast-spar stiffness to 100,000

---

## 2026-01-12: Phase 4 Refinement - Z-Height Layer System & Spar Implementation

### Summary
Refined the hurricane sail scenario with proper z-height-based layer separation and added a rigid spar connecting mast to sail.

### Key Learnings

#### 1. Z-Height Layer System
Particles now use `z_height` to control which types interact via SPH forces:
| z | Particles | Interactions |
|---|-----------|--------------|
| 0 | Water, Hull | Water↔Hull SPH |
| 1 | Mast, Spar | **Isolated** (bonds only, no SPH peers) |
| 2 | Air, Sail | Air↔Sail SPH |

- **Critical:** Without z-height filtering, air was bouncing off the hull and spar was exploding due to mast↔sail interactions.
- **Implementation:** Added `z_height` checks in `density.wgsl` and `forces.wgsl` with 0.5 threshold.

#### 2. Spar (Boom) Implementation
Added a rigid spar connecting mast to sail:
- **Structure:** 10x2 grid of mast particles at z=1 (isolated)
- **Bonds:** Horizontal, vertical, and diagonal for rigidity
- **Connections:** Mast center → Spar left column, Spar right column → Sail left edge

#### 3. Sail Tuning Parameters
| Parameter | Initial | Final | Notes |
|-----------|---------|-------|-------|
| Sail mass | 10 | 400 | Prevents explosion from air pressure |
| Sail stiffness | 1,000 | 15,000 | Rigid panel, not cloth |
| Sail shape | 8×6 grid | 2×10 line | Perpendicular to wind |
| Air-sail repulsion | 50,000 | 5,000 | Gentler wind interaction |
| Wind speed | 400 | 50 | Gentle test wind |
| Fuse stiffness | 5,000 | 30,000 | Strong mast-hull connection |

#### 4. Bond Stiffness Summary
| Type | Stiffness |
|------|-----------|
| Hull | 30,000 |
| Mast/Spar | 20,000 |
| Sail | 15,000 |
| Fuse (Mast-Hull) | 30,000 |
| Sail-Spar | 30,000 (2× sail) |

### Files Changed
- `src/resources.rs` - z_height values for air=2, sail=2, mast=1
- `src/simulation/scenarios.rs` - Spar spawning (10×2 grid), updated hurricane_config
- `src/simulation/setup.rs` - Spar bond generation, increased fuse stiffness
- `assets/shaders/density.wgsl` - z_height check before density accumulation
- `assets/shaders/forces.wgsl` - z_height check before force application, cleaned up layer logic

---

## 2026-01-12: Phase 4 - Cloth & Destruction Implementation

### Summary
Implemented flexible sails and breakable masts using Peridynamic bonds with per-type stiffness values.

### Key Additions

#### 1. Sail & Mast Particles
- Added `Particle::new_sail()` (mass=10, layer_mask=8) and `Particle::new_mast()` (mass=500, layer_mask=16) in `resources.rs`.
- Rendering colors: Sail=cream canvas, Mast=dark brown wood.

#### 2. Hurricane Test Scenario
- `scenario_hurricane()` in `scenarios.rs` creates:
  - Locked hull (single row, mass=100000)
  - Mast column (15 particles rising from hull center)
  - Sail grid (12x10 attached to mast)
  - Hurricane wind (400 px/s from left)

#### 3. Per-Type Bond Stiffness
- **Hull:** 30,000 (rigid)
- **Mast:** 20,000 (stiff)
- **Sail:** 1,000 (cloth-like flex, NO DIAGONALS for shear)
- **Fuse (Mast-Hull):** 5,000 with 0.3 breaking_strain (breaks under storm stress)

#### 4. Sail Aerodynamics
- Wind force: `F = 0.15 * (wind_vel - particle.vel)` (simplified relative velocity)
- Quadratic drag: `F = -0.3 * |v| * v`

### Files Changed
- `src/resources.rs` - new_sail(), new_mast() constructors
- `src/simulation/scenarios.rs` - scenario_hurricane(), hurricane_config module
- `src/simulation/setup.rs` - Sail/mast/fuse bond generation with tiered stiffness
- `assets/shaders/particles.wgsl` - Sail and mast particle colors
- `assets/shaders/forces.wgsl` - Sail aerodynamics section

### Testing
To verify: In `scenarios.rs`, change `spawn_particles()` to call `scenario_hurricane(particle_count)`, then `cargo run`.

---

## 2026-01-12: XSPH Same-Type Velocity Smoothing

### Summary
Fixed XSPH velocity smoothing to only occur between particles of the same type (water↔water, air↔air, hull↔hull).

### Problem
XSPH was averaging velocities between ALL particles that passed layer checks, including hull↔water interactions. This caused hull particles to be dragged by surrounding water flow, breaking rigid body behavior.

### Solution
Wrapped XSPH accumulation in the existing `same_layer` check:
```wgsl
if same_layer {
    let avg_density = (p.density + neighbor.density) * 0.5;
    let w = poly6_kernel(r_len * r_len, h);
    xsph_correction += (neighbor.mass / avg_density) * vel_diff * w;
}
```

### Key Learning
**XSPH is fluid-specific**: Velocity smoothing creates coherent motion within a fluid phase. Applying it across different materials (water, hull) causes unintended coupling. For rigid bodies like hulls, velocity coherence should come from bond forces, not XSPH.

### File Changed
- `assets/shaders/forces.wgsl` - Added `same_layer` guard around XSPH accumulation

---

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

## 2026-01-12: Phase 3 - Peridynamics Hull & Soft-Sphere Repulsion

### Summary
Implemented Peridynamic bonds to create a rigid hull structure, and added smooth soft-sphere repulsion to prevent water penetration.

### Key Learnings for Future Phases

#### 1. Atomic Force Accumulation for Bonds
- **Problem**: Multiple bonds act on the same particle, causing race conditions in GPU compute.
- **Solution**: Use atomic integer accumulation with fixed-point scaling: `atomicAdd(&forces[idx], i32(force * SCALER))`.
- **Critical**: The physics shader must ACTUALLY READ and APPLY these accumulated forces - don't just bind the buffer!

#### 2. Bond Force Application Was Missing
- **Bug**: `bonds.wgsl` wrote forces to atomic buffer, but `physics.wgsl` never read them.
- **Fix**: Added code in `physics.wgsl` to: load forces, convert from fixed-point, apply F/m*dt, then clear buffer.

#### 3. Layer Masks for Hull Particles
- **Problem**: Hull particles were receiving AIR_BUOYANCY force (7.0 upward every frame).
- **Cause**: Logic only checked "is_water" and applied buoyancy to everything else.
- **Fix**: Explicitly check `is_hull = (layer_mask & 4) != 0` and exclude from gravity/buoyancy.

#### 4. Lennard-Jones Potential Explodes (Don't Use!)
- **Problem**: LJ formula `F = D * [(r0/r)^4 - (r0/r)^2]` causes particles to shoot off.
- **Cause**: When `r → 0`, the `1/r^n` terms explode to infinity.
- **Lesson**: Even with tiny strength (0.1), if particles start inside hull, they get astronomical forces.

#### 5. Soft-Sphere Repulsion (Use This!)
- **Solution**: Use bounded linear or quadratic repulsion instead of LJ.
- **Linear**: `F = k * (r0 - r)` when `r < r0` - bounded but has discontinuous derivative.
- **Quadratic (smooth)**: `F = k * (1 - r/r0)²` - both force AND derivative are 0 at threshold.
- **Benefit**: Maximum force is always `k` (bounded), gracefully handles overlapping particles.

#### 6. Don't Spawn Particles Inside Rigid Bodies
- **Problem**: Even with soft-sphere, initial overlaps cause high forces.
- **Fix**: Calculate hull bounding box (with margin) and skip spawning water inside it.

#### 7. Bond Damping Prevents Oscillation
- **Problem**: Stiff springs without damping oscillate forever.
- **Solution**: Add velocity-based damping: `F_damp = c * dot(v_rel, spring_dir)`.
- **Caution**: Very high stiffness (>1M) can still cause instability or integer overflow.

### Files Changed
- `assets/shaders/bonds.wgsl` - NEW: Peridynamic bond force computation
- `assets/shaders/bond_lines.wgsl` - NEW: Bond visualization shader
- `assets/shaders/physics.wgsl` - Bond force application, layer-aware gravity
- `assets/shaders/forces.wgsl` - Soft-sphere repulsion, XSPH smoothing enabled
- `src/simulation/setup.rs` - Hull particle spawning, bond generation, hull exclusion zone
- `src/render/mod.rs` - Bond line rendering pipeline

## 2026-01-12: Scenario System & Physics Cleanup

### Summary
Implemented a scenario selection system and removed vertical gravity/buoyancy to align with the Top-Down 2D simulation model.

### Key Changes
1.  **Scenario System**: Created `src/simulation/scenarios.rs` to manage different test cases (`dry_dock`, `pressure_washer`, etc.). Switching scenarios is now a single function call.
2.  **Gravity/Buoyancy Removal**: Removed `AIR_BUOYANCY` and vertical gravity application from shaders. In a top-down view, "up" (Y+) is North, not skyward, so vertical gravity was incorrect.
3.  **Lava Lamp Obsolescence**: Removed the "Lava Lamp" scenario as it relied on vertical density separation which no longer exists.

### Files Changed
- `src/simulation/scenarios.rs` - NEW: Scenario management
- `src/simulation/setup.rs` - Updated to use scenario system
- `assets/shaders/forces.wgsl` - Removed buoyancy constants and logic
- `assets/shaders/physics.wgsl` - Removed gravity constants (if any remaining)
- `WORK_PLAN.md` - Updated to mark Lava Lamp as obsolete

---

### Key Learnings for Future Phases

#### 1. Hull Repulsion Tuning (The "Nuclear Option")
- **Problem**: fast-moving Air particles (Wind) were tunneling through the Hull even with `500k` quadratic repulsion.
- **Solution**:
    - Switch to **Linear Ramp**: `F = k * (1 - r/r0)`. This provides immediate stiff resistance at `r = 0.9*r0`, unlike quadratic which is weak at the edge.
    - **Strength**: Increased to `2,000,000.0`.
    - **Radius**: Increased to `20.0` (4x particle spacing) to engage particles earlier.
    - **Logic Fix**: Ensured repulsion applies to *any* non-hull fluid (Air/Water), verifying `!same_layer && (is_hull || neighbor_is_hull)`.
    - **Refinement (Dual-Mode)**: The high stiff settings caused low-speed Water to explode.
        - **Air**: Uses **Linear** ramp, `k=2M`, `r=20.0` (Stiff Wall).
        - **Water**: Uses **Quadratic** ramp, `k=100k`, `r=12.0` (Soft Buffer).
        - This ensures Air bounces off but Water settles stably against the hull.

#### 2. Air Pressure Activation
- **Problem**: Air stream was overly collimated / not spreading out.
- **Cause**: `target_density_air` (0.5) was too high for Air Mass (1.0). Actual density was ~0.02, resulting in zero pressure (clamped).
- **Solution**: Reduced `target_density_air` to **0.02**. This activates SPH repulsive pressure, making the air actually behave like a fluid/gas and spread out.

#### 3. Water Spreading
- **Request**: User wanted water to "spread out a little bit more".
- **Solution**: Reduced `target_density_water` from `1.0` to **0.8**. This effectively increases the volume per particle by ~25%, causing the water to occupy more space.
