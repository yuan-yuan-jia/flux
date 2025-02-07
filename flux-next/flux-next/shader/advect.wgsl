// include fluid.inc
struct FluidUniforms {
  timestep: f32,
  dissipation: f32,
  texel_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: FluidUniforms;
@group(0) @binding(1) var velocity_texture: texture_2d<f32>;
@group(0) @binding(2) var velocity_sampler: sampler;
@group(0) @binding(3) var out_texture: texture_storage_2d<rg32float, write>;

// TODO: reduce size
var<push_constant> direction: f32;

// TODO: determine workgroup size
@compute
@workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    // TODO: verify coordinates here
    let texel_position = vec2<i32>(global_id.xy);
    let velocity = textureLoad(velocity_texture, texel_position, 0).xy;
    // Note, that, by multiplying by dx, we’ve “incorrectly” scaled our coordinate system.
    // This is actually a key component of the slow, wriggly “coral reef” look.
    let size = textureDimensions(velocity_texture);
    let decay = 1.0 + uniforms.dissipation * direction * uniforms.timestep;
    let advected_position = ((vec2<f32>(texel_position) + 0.5) - direction * uniforms.timestep * velocity) / vec2<f32>(size);
    let new_velocity = textureSampleLevel(velocity_texture, velocity_sampler, advected_position, 0.0).xy / decay;
    textureStore(out_texture, texel_position, vec4<f32>(new_velocity, 0., 0.));
}
