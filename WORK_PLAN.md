## Implementation Roadmap

The agent must follow this strictly. Do not jump ahead.

### Phase 0: The GPU Boilerplate

**Goal:** A Bevy app that launches a Compute Shader, updates a buffer, and renders dots.
**Tasks:**

* [x] Initialize `cargo new` with `bevy`, `bytemuck`.
* [x] Set up the `MainCamera` (2D).
* [x] Create the `ParticleBuffer` (Storage Buffer) initialized with random positions.
* [x] Create a basic Compute Shader (`update.wgsl`) that moves particles: `pos += vel * dt`.
* [x] Create a Render Pipeline that draws instances of a quad/circle at `particle.pos`.
* [x] **Verification:** You see thousands of dots drifting across the screen.

### Phase 1: The Stability Sandbox (Multi-Phase Fluid)

**Goal:** Stable interaction between "Air" and "Water" particles.
**Physics Model:** Weakly Compressible SPH (WCSPH).
**Tasks:**

* [x] **Spatial Indexing:** Implement **Bitonic Sort**.
  * Shader 1: Calculate `cell_id` for every particle based on grid position.
  * Shader 2: Sort the particle buffer by `cell_id`.
  * Shader 3: Build a `CellStartEnd` buffer (mapping grid ID to start/end indices in the sorted array).
  * *Note: Currently using O(n²) brute force for correctness; grid-based sorting stubbed for future optimization.*


* [x] **Density Kernel:** Compute density  for each particle by summing neighbors within `smoothing_radius`.
  * *Constraint:* Tune `mass` such that Air is light and Water is heavy. Use "Hollywood Ratios" (Air = 1, Water = 10) to prevent explosion, not real physics (1:1000).


* [x] **Force Kernel:** Compute Pressure Force () and Viscosity.
  * Use Tait’s Equation of State: .


* [x] **Integration:** Update Velocity and Position. Apply Gravity ().
* [x] **Boundaries:** Implement a "Reflection" boundary at the screen edges (keep particles inside).
* [x] **Verification Scenario ("Lava Lamp"):** [OBSOLETE] Removed in Phase 3. Simulation is Top-Down 2D, so vertical gravity/buoyancy separation is not applicable.

### Phase 2: The 2.5D Layer Logic

**Goal:** Implement the height-based collision filtering.
**Tasks:**

* [x] **Z-Axis logic:** In the Force Kernel, check `layer_mask` and `z_height` before applying forces.
* `if (pA.layer == WATER && pB.layer == AIR)`: Only interact if `pA.density > threshold` (Wave Crest).


* [x] **The Wind Tunnel:** Create an "Emitter" system.
* When an Air particle leaves the Right edge of the screen, teleport it to the Left edge.
* Maintain a constant flow of wind.


* [x] **Verification Scenario ("Pressure Washer"):** Create a vertical line of static particles (Wall). Blast wind at it. Wind should bounce off.
* *Update:* Wall made 3x denser (spacing 5.0). Repulsion strength increased 5x (500k) and logic fixed to apply to Air particles.

### Phase 3: The Peridynamic Hull

**Goal:** A floating rigid body.
**Tasks:**

* [x] **Bond Logic:** Implement a Compute Shader kernel that iterates over the `BondBuffer`.
* Calculate distance.
* Force using Hooke's law with damping.
* Apply forces atomically to particle A and particle B.
* *Note: Uses fixed-point integer accumulation to avoid race conditions.*


* [x] **Hull Generation (CPU):** Create a function `spawn_hull(x, y)` that generates a grid of particles.
* Add **Diagonal Bonds** (Cross-bracing) to make it rigid.
* Set `stiffness` high (30,000).
* Exclude hull bounding box from water spawning.


* [x] **Soft-Sphere Repulsion:** Prevent water from penetrating hull.
* Smooth quadratic repulsion: `F = k * (1 - r/r0)²`
* Applied between Hull↔Water particles only.
* *Note: Lennard-Jones potential was abandoned due to singularity at r→0.*


* [x] **XSPH Same-Type Smoothing:** Fixed XSPH to only smooth velocities within same particle type.


* [x] **Verification:**
    * Create a Hull. Place it in Water (`dry_dock`).
    * **Result:** Hull is effectively rigid (Bonds). Water does not penetrate (Dual-Mode Repulsion).
    * *Note:* "Drop test" irrelevant without gravity. Containment verified.

### Phase 4: Cloth & Destruction

**Goal:** Flexible sails and breakable masts.
**Tasks:**

* [x] **Sail Generation:** Generate a grid of particles with bonds for sail structure.
  * *Final:* 2×10 sail grid (vertical line perpendicular to wind) with diagonal bonds for rigidity.
  * *Mass:* 400 (heavy for stability against wind pressure).

* [x] **Spar (Boom) Implementation:** Rigid beam connecting mast to sail.
  * *Structure:* 10×2 grid of mast particles at z=1 (isolated from air).
  * *Bonds:* Horizontal, vertical, and diagonal for full rigidity.
  * *Connections:* Mast center → Spar left column, Spar right column → Sail left edge.

* [x] **Z-Height Layer System:** Particles filtered by z_height for SPH interactions.
  * z=0: Water, Hull (interact via SPH)
  * z=1: Mast, Spar (isolated - bonds only)
  * z=2: Air, Sail (interact via SPH)
  * *Critical:* Prevents air bouncing off hull and mast exploding with sail.

* [x] **Bond Stiffness Tuning:**
  * Hull: 30,000 | Mast/Spar: 20,000 | Sail: 15,000 | Fuse: 30,000

* [x] **Fracture Logic:**
  * Already implemented in `bonds.wgsl`.
  * Fuse breaking_strain: 2.0 (non-breaking for normal operation).

* [x] **Aerodynamics:** Apply simplified Drag to Sail particles.
  * Air-sail repulsion: 5,000 strength, 10.0 radius (gentle).
  * Wind speed: 50 px/s (gentle test wind).

* [x] **Verification Scenario ("The Hurricane"):**
  * `scenario_hurricane()` in `scenarios.rs`.
  * To test: Change `spawn_particles()` to call `scenario_hurricane(particle_count)`.


### Phase 5: Gameplay & Controls

**Goal:** Interactive steering.
**Tasks:**

* [ ] **Rudder:** Identify Rudder particles. Rotate their relative positions based on `SimParams.rudder_angle`. The bonds will force the physical particles to follow.
* [ ] **Sheets:** Identify Sheet bonds. Dynamically update their `rest_length` based on `SimParams.sheet_extension`.
* [ ] **Camera:** Implement a smooth 2D camera following the Hull Center of Mass.