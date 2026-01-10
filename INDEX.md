src/
  main.rs          // Bevy setup, State machine
  resources.rs     // GPU struct definitions (Pod/Zeroable)
  simulation/
    mod.rs         // Plugin definition
    setup.rs       // Buffer initialization
    systems.rs     // Dispatch systems
  render/
    mod.rs         // Custom material/pipeline
assets/
  shaders/
    particles.wgsl // Vertex/Fragment shaders
    physics.wgsl   // SPH solver
    sort.wgsl      // Bitonic sort
    bonds.wgsl     // Peridynamics