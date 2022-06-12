#ifdef GL_ES
precision highp float;
precision highp sampler2D;
#endif

layout(std140) uniform FluidUniforms
{
  highp float deltaT;
  highp float dissipation;
  highp vec2 uTexelSize;
};

uniform sampler2D velocityTexture;
uniform float amount;

in vec2 texturePosition;
out vec2 outVelocity;

void main() {
  vec2 velocity = texture(velocityTexture, texturePosition).xy;
  // Note, that, by multiplying by 1/dx, we’ve “incorrectly” scaled our coordinate system.
  // This is actually a key component of the slow, wriggly “coral reef” look.
  vec2 advectedPosition = (texturePosition + 0.5 * uTexelSize) - uTexelSize * amount * velocity;
  float decay = 1.0 + dissipation * amount;
  outVelocity = texture(velocityTexture, advectedPosition).xy / decay;
}
