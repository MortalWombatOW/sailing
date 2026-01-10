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
