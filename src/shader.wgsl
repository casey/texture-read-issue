@group(0)
@binding(0)
var t: texture_2d<f32>;

@group(0)
@binding(1)
var s: sampler;

@group(0)
@binding(2)
var<uniform> uniforms: Uniforms;

struct Uniforms {
  i: u32,
  resolution: f32,
}

const VERTICES = array(
  vec4(-1.0, -1.0, 0.0, 1.0),
  vec4(3.0, -1.0, 0.0, 1.0),
  vec4(-1.0, 3.0, 0.0, 1.0)
);

@vertex
fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
  return VERTICES[i];
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
  let uv = position.xy / uniforms.resolution;
  let input = textureSample(t, s, uv);
  var on: bool;
  switch uniforms.i {
    case 0u {
      return vec4(input.xyz * -1 + 1, 1.0);
    }
    case 1u {
      return input;
    }
    default {
      return vec4(0.0, 1.0, 0.0, 1.0);
    }
  }
}
