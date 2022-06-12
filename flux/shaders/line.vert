#ifdef GL_ES
precision highp float;
#endif

in vec2 lineVertex;
in vec2 basepoint;

in vec2 iEndpointVector;
in vec2 iVelocityVector;
in mediump vec4 iColor;
in mediump float iLineWidth;

layout(std140) uniform LineUniforms
{
  highp float aspect;
  highp float zoom;
  mediump float uLineWidth;
  mediump float uLineLength;
  mediump float uLineBeginOffset;
  mediump float uLineVariance;
  highp float lineNoiseOffset1;
  highp float lineNoiseOffset2;
  highp float lineNoiseBlendFactor;
  highp float deltaTime;
};

out vec2 vVertex;
out vec4 vColor;
out float vLineOffset;

void main() {
  vec2 xBasis = vec2(-iEndpointVector.y, iEndpointVector.x);
  xBasis /= length(xBasis) + 0.0001; // safely normalize

  vec2 point =
    vec2(aspect, 1.0) * zoom * (basepoint * 2.0 - 1.0)
    + iEndpointVector * lineVertex.y
    + uLineWidth * iLineWidth * xBasis * lineVertex.x;

  point.x /= aspect;

  gl_Position = vec4(point, 0.0, 1.0);
  vVertex = lineVertex;
  vColor = iColor;

  float shortLineBoost = 1.0 + (uLineWidth * iLineWidth) / length(iEndpointVector);
  vLineOffset = uLineBeginOffset / shortLineBoost;
}
