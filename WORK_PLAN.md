## Implementation Roadmap

The agent must follow this strictly. Do not jump ahead.

### Phase 0: The GPU Boilerplate

**Goal:** A Bevy app that launches a Compute Shader, updates a buffer, and renders dots.
**Tasks:**

* [ ] Initialize `cargo new` with `bevy`, `bytemuck`.
* [ ] Set up the `MainCamera` (2D).
* [ ] Create the `ParticleBuffer` (Storage Buffer) initialized with random positions.
* [ ] Create a basic Compute Shader (`update.wgsl`) that moves particles: `pos += vel * dt`.
* [ ] Create a Render Pipeline that draws instances of a quad/circle at `particle.pos`.
* [ ] **Verification:** You see thousands of dots drifting across the screen.

### Phase 1: The Stability Sandbox (Multi-Phase Fluid)

**Goal:** Stable interaction between "Air" and "Water" particles.
**Physics Model:** Weakly Compressible SPH (WCSPH).
**Tasks:**

* [ ] **Spatial Indexing:** Implement **Bitonic Sort**.
* Shader 1: Calculate `cell_id` for every particle based on grid position.
* Shader 2: Sort the particle buffer by `cell_id`.
* Shader 3: Build a `CellStartEnd` buffer (mapping grid ID to start/end indices in the sorted array).


* [ ] **Density Kernel:** Compute density  for each particle by summing neighbors within `smoothing_radius`.
* *Constraint:* Tune `mass` such that Air is light and Water is heavy. Use "Hollywood Ratios" (Air = 1, Water = 10) to prevent explosion, not real physics (1:1000).


* [ ] **Force Kernel:** Compute Pressure Force () and Viscosity.
* Use Tait’s Equation of State: .


* [ ] **Integration:** Update Velocity and Position. Apply Gravity ().
* [ ] **Boundaries:** Implement a "Reflection" boundary at the screen edges (keep particles inside).
* [ ] **Verification Scenario ("Lava Lamp"):** Initialize a box with mixed Air/Water. They must separate cleanly. Water at bottom, Air on top. No explosions.

### Phase 2: The 2.5D Layer Logic

**Goal:** Implement the height-based collision filtering.
**Tasks:**

* [ ] **Z-Axis logic:** In the Force Kernel, check `layer_mask` and `z_height` before applying forces.
* `if (pA.layer == WATER && pB.layer == AIR)`: Only interact if `pA.density > threshold` (Wave Crest).


* [ ] **The Wind Tunnel:** Create an "Emitter" system.
* When an Air particle leaves the Right edge of the screen, teleport it to the Left edge.
* Maintain a constant flow of wind.


* [ ] **Verification Scenario ("Pressure Washer"):** Create a vertical line of static particles (Wall). Blast wind at it. Wind should bounce off.

### Phase 3: The Peridynamic Hull

**Goal:** A floating rigid body.
**Tasks:**

* [ ] **Bond Logic:** Implement a Compute Shader kernel that iterates over the `BondBuffer`.
* Calculate distance .
* Force .
* Apply  to particle A and  to particle B.


* [ ] **Hull Generation (CPU):** Create a function `spawn_hull(x, y)` that generates a grid of particles.
* Add **Diagonal Bonds** (Cross-bracing) to make it rigid.
* Set `stiffness` high.


* [ ] **Buoyancy:** The hull particles interact with Water particles via standard SPH pressure (repulsion).
* *Tuning:* You must adjust the Hull Particle Mass so it settles at the correct waterline (neutral buoyancy).


* [ ] **Verification Scenario ("Dry Dock"):** Drop the hull into the water. It should splash, bob, and settle.

### Phase 4: Cloth & Destruction

**Goal:** Flexible sails and breakable masts.
**Tasks:**

* [ ] **Sail Generation:** Generate a grid of particles with **Structural Bonds Only** (No diagonals). This allows shear/folding (Cloth behavior).
* [ ] **Bond Types:** Update Bond Kernel to use different stiffness for Hull vs. Sail.
* [ ] **Fracture Logic:**
* Calculate Strain: .
* If `s > breaking_strain`, set `bond.active = 0`.
* Multiply force by `bond.active`.


* [ ] **Aerodynamics:** Apply simplified Drag to Sail particles based on Wind Velocity relative to Sail Normal.
* [ ] **Verification Scenario ("The Hurricane"):** Lock the hull. Increase wind speed. The Sail should billow, then the Mast-Hull bonds should snap.

### Phase 5: Gameplay & Controls

**Goal:** Interactive steering.
**Tasks:**

* [ ] **Rudder:** Identify Rudder particles. Rotate their relative positions based on `SimParams.rudder_angle`. The bonds will force the physical particles to follow.
* [ ] **Sheets:** Identify Sheet bonds. Dynamically update their `rest_length` based on `SimParams.sheet_extension`.
* [ ] **Camera:** Implement a smooth 2D camera following the Hull Center of Mass.