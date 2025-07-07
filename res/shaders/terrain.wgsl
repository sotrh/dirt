struct TerrainData {
    terrain_height__tile_size: vec2<f32>,
}

@group(0)
@binding(0)
var<uniform> terrain_data: TerrainData;

struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(1)
@binding(0)
var<uniform> camera: CameraUniform;

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
    world_normal: vec3<f32>
}

@vertex
fn displace_terrain(
   @builtin(vertex_index) index: u32,
   instance: TileInstance,
) -> VsOut {
    let i = f32(index);
    let x = i % terrain_data.terrain_height__tile_size.y + instance.tile_offset.x;
    let z = i / terrain_data.terrain_height__tile_size.y + instance.tile_offset.y;

    var world_position = vec3(x, 0.0, z);

    world_position.y = height_map(world_position.xz);

    let world_normal = vec3(0.0, 1.0, 0.0);
    let frag_position = camera.view_proj * vec4(world_position, 1.0);
    let f = world_position.y / terrain_data.terrain_height__tile_size.x;
    let debug = vec3(f);

    return VsOut(
        frag_position,
        debug,
        world_position,
        world_normal,
    );
}

@fragment
fn debug(vs: VsOut) -> @location(0) vec4<f32> {
    return vec4(vs.debug, 1.0);
}

@fragment
fn triplanar_shaded(vs: VsOut) -> @location(0) vec4<f32> {
    

    return vec4(fract(vs.world_position * 0.1), 1.0);
}

fn height_map(p: vec2<f32>) -> f32 {
    return (fbm(p) * 0.5 + 0.5) * terrain_data.terrain_height__tile_size.x;
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
    // I flipped the condition here from > to < as it fixed some artifacting I was observing
    var i1: vec2<f32> = select(vec2<f32>(1., 0.), vec2<f32>(0., 1.), (x0.x < x0.y));
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