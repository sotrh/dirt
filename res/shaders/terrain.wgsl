struct TerrainData {
    terrain_height__tile_size: vec2<f32>,
}

struct TerrainVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
}

@group(0)
@binding(0)
var<uniform> terrain_data: TerrainData;

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(1)
@binding(0)
var<uniform> camera: CameraUniform;

@group(2)
@binding(0)
var terrain_textures: texture_2d_array<f32>;
@group(2)
@binding(1)
var terrain_sampler: sampler;

struct TileInstance {
    @location(0)
    tile_offset: vec2<f32>,
}

struct VsOut {
    @builtin(position)
    frag_position: vec4<f32>,
    @location(0)
    debug: vec3<f32>,
    @location(1)
    world_position: vec3<f32>,
    @location(2)
    world_normal: vec3<f32>,
}

@vertex
fn displace_terrain(
   @builtin(vertex_index) index: u32,
   instance: TileInstance,
) -> VsOut {
    let i = f32(index);
    let x = i % terrain_data.terrain_height__tile_size.y + instance.tile_offset.x;
    let z = i / terrain_data.terrain_height__tile_size.y + instance.tile_offset.y;

    let v = terrain_vertex(vec2(x, z));

    let frag_position = camera.view_proj * vec4(v.position, 1.0);
    let f = v.position.y / terrain_data.terrain_height__tile_size.x;
    let debug = v.normal.xyz * 0.5 + 0.5;

    return VsOut(
        frag_position,
        debug,
        v.position,
        v.normal,
    );
}

@fragment
fn debug(vs: VsOut) -> @location(0) vec4<f32> {
    return vec4(vs.debug, 1.0);
}

@fragment
fn triplanar_shaded(vs: VsOut) -> @location(0) vec4<f32> {
    // Adapted from https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a
    var vs_world_normal = normalize(vs.world_normal);

    let cos_theta = max(dot(vs_world_normal, vec3(0.0, 1.0, 0.0)), 0.0);
    let layer = select(2u, 0u, cos_theta > 0.8);

    var blend = abs(vs_world_normal);
    blend /= blend.x + blend.y + blend.z;

    let uv_x = vs.world_position.zy * 0.1;
    let uv_y = vs.world_position.xz * 0.1;
    let uv_z = vs.world_position.xy * 0.1;
    
    let albedo_x = to_linear(textureSample(terrain_textures, terrain_sampler, uv_x, layer).rgb);
    let albedo_y = to_linear(textureSample(terrain_textures, terrain_sampler, uv_y, layer).rgb);
    let albedo_z = to_linear(textureSample(terrain_textures, terrain_sampler, uv_z, layer).rgb);
    let albedo = albedo_x * blend.x + albedo_y * blend.y + albedo_z * blend.z;

    var tnormal_x = 2.0 * textureSample(terrain_textures, terrain_sampler, uv_x, layer + 1u).xyz - 1.0;
    var tnormal_y = 2.0 * textureSample(terrain_textures, terrain_sampler, uv_y, layer + 1u).xyz - 1.0;
    var tnormal_z = 2.0 * textureSample(terrain_textures, terrain_sampler, uv_z, layer + 1u).xyz - 1.0;

    tnormal_x = vec3(
        tnormal_x.xy + vs_world_normal.zy,
        abs(tnormal_x.z) * vs_world_normal.x,
    );
    tnormal_y = vec3(
        tnormal_y.xy + vs_world_normal.xz,
        abs(tnormal_y.z) * vs_world_normal.y,
    );
    tnormal_z = vec3(
        tnormal_z.xy + vs_world_normal.xy,
        abs(tnormal_z.z) * vs_world_normal.z,
    );

    let world_normal = normalize(
        tnormal_x.zyx * blend.x +
        tnormal_y.xzy * blend.y +
        tnormal_z.xyz * blend.z
    );

    let ambient_strength = 0.1;
    let ambient_color = vec3(1.0) * ambient_strength;

    // let light_dir = normalize(light.position - vs.world_position);
    let light_dir = normalize(vec3(1.0, 1.0, 1.0));
    let view_dir = normalize(camera.view_pos.xyz - vs.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse_strength = max(dot(world_normal, light_dir), 0.0);
    let diffuse_color = diffuse_strength * vec3(1.0, 1.0, 1.0);

    // let specular_strength = pow(max(dot(world_normal, half_dir), 0.0), 16.0);
    let specular_strength = 0.0;
    let specular_color = specular_strength * vec3(1.0, 1.0, 1.0);

    let result = (ambient_color + diffuse_color + specular_color) * albedo.rgb;
    // let result = (ambient_color + diffuse_color) * albedo.rgb;
    // let result = albedo.rgb;
    // let result = blend;

    return vec4(result, 1.0);
}

fn to_srgb(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3(0.0031308);
    let higher = vec3(1.055) * pow(rgb, vec3(1.0 / 2.4)) - vec3(0.055);
    let lower = rgb * vec3(12.92);

    return select(higher, lower, cutoff);
}

fn to_linear(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3(0.04045);
    let higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    let lower = srgb / vec3(12.92);

    return select(higher, lower, cutoff);
}

fn terrain_vertex(p: vec2<f32>) -> TerrainVertex {
    let v = terrain_point(p);

    let tpx = terrain_point(p + vec2<f32>(0.1, 0.0)) - v;
    let tnx = terrain_point(p + vec2<f32>(-0.1, 0.0)) - v;
    let tpz = terrain_point(p + vec2<f32>(0.0, 0.1)) - v;
    let tnz = terrain_point(p + vec2<f32>(0.0, -0.1)) - v;

    let pn = normalize(cross(tpz, tpx));
    let nn = normalize(cross(tnz, tnx));

    let n = (pn + nn) * 0.5;

    return TerrainVertex(v, n);
}

fn terrain_point(p: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(
        p.x,
        (fbm(p) * 0.5 + 0.5) * terrain_data.terrain_height__tile_size.x,
        p.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    // TODO: add this to uniforms
    let NUM_OCTAVES: u32 = 5u;
    var x = p * 0.01;
    var v = 0.0;
    var a = 0.5;
    let shift = vec2<f32>(100.0);
    let cs = vec2<f32>(cos(0.5), sin(0.5));
    let rot = mat2x2<f32>(cs.x, cs.y, -cs.y, cs.x);

    for (var i = 0u; i < NUM_OCTAVES; i = i + 1u) {
        v = v + a * snoise2(x);
        x = rot * x * 2.0 + shift;
        a = a * 0.5;
    }

    return v;
}

// https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket
// - Less condensed glsl implementation with comments can be found at https://weber.itn.liu.se/~stegu/jgt2012/article.pdf

fn permute3(x: vec3<f32>) -> vec3<f32> { return (((x * 34.) + 1.) * x) % vec3<f32>(289.); }

fn snoise2(v: vec2<f32>) -> f32 {
    let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    var i: vec2<f32> = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);
    var i1: vec2<f32> = select(vec2<f32>(0., 1.), vec2<f32>(1., 0.), (x0.x > x0.y));
    var x12: vec4<f32> = x0.xyxy + C.xxzz - vec4<f32>(i1, 0., 0.);
    i = i % vec2<f32>(289.);
    let p = permute3(permute3(i.y + vec3<f32>(0., i1.y, 1.)) + i.x + vec3<f32>(0., i1.x, 1.));
    var m: vec3<f32> = max(0.5 - vec3<f32>(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3<f32>(0.));
    m = m * m;
    m = m * m;
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;
    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));
    let g = vec3<f32>(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return 130. * dot(m, g);
}