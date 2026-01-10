
## Critical Engineering Guidelines (Do not ignore)

### 1. Stability Measures

* **Time Step:** Use a fixed time step. Start with `dt = 0.005` (5ms). If instability occurs, lower it, or implement **Sub-stepping** (run physics 4x per frame at `dt = 0.00125`).
* **Damping:** You MUST implement XSPH Viscosity or simple Linear Damping () to prevent energy accumulation (Explosions).
* **Particle Radius:** In the physics calc, use a smoothing radius  that is slightly larger than the visual particle size to prevent "Tunneling" (leaks).

### 2. Optimization

* **Sorting:** The Bitonic Sort is complex to implement. If you get stuck, a "Radix Sort" is also acceptable. **Do not fall back to  brute force.**
* **Workgroups:** Use a workgroup size of 64 or 256 for compute shaders.

### 3. Visualization

* **Color Coding:** For debugging, color particles by:
* `Type`: Water=Blue, Air=White, Hull=Brown, Sail=Red.
* `Velocity`: Dark=Slow, Bright=Fast.
* `Density`: Useful to visualize pressure buildup.

