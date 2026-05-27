//! WGSL shader sources — embedded at compile time, uploaded to WebGPU at runtime.
//!
//! All shaders expect the bind-group layout described in `pipeline.rs`.

// ─── Voxel world shader ───────────────────────────────────────────────────────

/// Phong-lit voxel shader with fake ambient-occlusion and distance fog.
///
/// ## Bind groups
/// - `@group(0) @binding(0)` — `Camera` uniform (view_proj, position)
/// - `@group(0) @binding(1)` — `WorldUniforms` (sun_dir, time, fog_*)
///
/// ## Vertex layout
/// Matches `GpuVoxelVertex` in `pipeline.rs`:
/// - `@location(0)` — `position : vec3<f32>`  (offset  0)
/// - `@location(1)` — `normal   : vec3<f32>`  (offset 12)
/// - `@location(2)` — `color    : vec4<f32>`  (offset 24)
pub const VOXEL_SHADER: &str = r#"
struct Camera {
    view_proj : mat4x4<f32>,
    position  : vec3<f32>,
    _pad      : f32,
};
struct WorldUniforms {
    sun_dir   : vec3<f32>,
    time      : f32,
    fog_color : vec4<f32>,
    fog_start : f32,
    fog_end   : f32,
    _pad      : vec2<f32>,
};

@group(0) @binding(0) var<uniform> cam   : Camera;
@group(0) @binding(1) var<uniform> world : WorldUniforms;

struct VIn  {
    @location(0) pos : vec3<f32>,
    @location(1) nrm : vec3<f32>,
    @location(2) col : vec4<f32>,
};
struct VOut {
    @builtin(position) clip : vec4<f32>,
    @location(0) wpos : vec3<f32>,
    @location(1) nrm  : vec3<f32>,
    @location(2) col  : vec4<f32>,
};

@vertex
fn vs_main(v: VIn) -> VOut {
    return VOut(
        cam.view_proj * vec4(v.pos, 1.0),
        v.pos, v.nrm, v.col
    );
}

@fragment
fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    let n       = normalize(v.nrm);
    let sun     = normalize(world.sun_dir);
    let diffuse = max(dot(n, sun), 0.0);

    // Fake AO: bottom faces are darkened proportionally.
    let ao      = mix(1.0, 0.65, max(0.0, -v.nrm.y));

    let lit_rgb = v.col.rgb * (0.28 + diffuse * 0.72) * ao;

    // Distance fog
    let view_dist = length(cam.position - v.wpos);
    let fog_t     = clamp(
        (view_dist - world.fog_start) / (world.fog_end - world.fog_start),
        0.0, 1.0
    );
    let final_rgb = mix(lit_rgb, world.fog_color.rgb, fog_t);

    return vec4(final_rgb, v.col.a);
}
"#;

// ─── Sky shader ───────────────────────────────────────────────────────────────

/// Sky-dome gradient shader.
///
/// Rendered at maximum depth using the `pos.xyww` Z-trick so it always
/// appears behind all other geometry without writing to the depth buffer.
///
/// ## Bind groups
/// - `@group(0) @binding(0)` — `Camera` uniform
/// - `@group(1) @binding(0)` — `SkyUniforms` (top, horizon colors + time)
pub const SKY_SHADER: &str = r#"
struct Camera    { view_proj:mat4x4<f32>, position:vec3<f32>, _pad:f32 };
struct SkyUniforms {
    sky_top     : vec4<f32>,
    sky_horizon : vec4<f32>,
    time        : f32,
    _pad        : vec3<f32>,
};

@group(0) @binding(0) var<uniform> cam : Camera;
@group(1) @binding(0) var<uniform> sky : SkyUniforms;

struct SkyIn  { @location(0) pos : vec3<f32> };
struct SkyOut { @builtin(position) clip:vec4<f32>, @location(0) dir:vec3<f32> };

@vertex
fn vs_sky(v: SkyIn) -> SkyOut {
    // Translate with camera so the sky is always centred.
    let world_pos = v.pos + cam.position;
    let clip = cam.view_proj * vec4(world_pos, 1.0);
    return SkyOut(clip.xyww, v.pos);   // xyww sets z = w → maximum depth
}

@fragment
fn fs_sky(v: SkyOut) -> @location(0) vec4<f32> {
    let t = clamp(normalize(v.dir).y * 0.5 + 0.5, 0.0, 1.0);
    return mix(sky.sky_horizon, sky.sky_top, t);
}
"#;

// ─── UI / HUD shader ──────────────────────────────────────────────────────────

/// Unlit, alpha-blended 2-D UI shader for the HUD overlay.
///
/// Vertices are in **logical CSS pixels** — the uniform converts them to NDC.
///
/// ## Bind groups
/// - `@group(0) @binding(0)` — `UIUniforms` (screen_size)
/// - `@group(0) @binding(1)` — texture_2d sampler (font / sprite atlas)
pub const UI_SHADER: &str = r#"
struct UIUniforms { screen_size : vec2<f32>, _pad : vec2<f32> };

@group(0) @binding(0) var<uniform>  ui      : UIUniforms;
@group(0) @binding(1) var           t_color : texture_2d<f32>;
@group(0) @binding(2) var           s_color : sampler;

struct UIVert {
    @location(0) pos : vec2<f32>,
    @location(1) uv  : vec2<f32>,
    @location(2) col : vec4<f32>,
};
struct UIOut {
    @builtin(position) clip : vec4<f32>,
    @location(0) uv  : vec2<f32>,
    @location(1) col : vec4<f32>,
};

@vertex
fn vs_ui(v: UIVert) -> UIOut {
    // Map CSS pixel coords to NDC [-1, 1], flip Y.
    let ndc = v.pos / ui.screen_size * 2.0 - vec2(1.0, 1.0);
    return UIOut(vec4(ndc.x, -ndc.y, 0.0, 1.0), v.uv, v.col);
}

@fragment
fn fs_ui(v: UIOut) -> @location(0) vec4<f32> {
    return textureSample(t_color, s_color, v.uv) * v.col;
}
"#;
