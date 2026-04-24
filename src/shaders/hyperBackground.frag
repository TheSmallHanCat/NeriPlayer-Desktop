precision highp float;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_musicLevel;
uniform float u_beat;
uniform vec4 u_color0;
uniform vec4 u_color1;
uniform vec4 u_color2;
uniform vec4 u_color3;
uniform float u_darkMode;
uniform float u_lightOffset;
uniform float u_saturateOffset;

// --- 色彩空间工具（对齐 Android hyper_background_effect.glsl） ---

vec3 rgb2hsv(vec3 c) {
  vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
  vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
  vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
  float d = q.x - min(q.w, q.y);
  float e = 1.0e-10;
  return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv2rgb(vec3 c) {
  vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
  return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

// Perlin 噪声（对齐 Android）
float hash(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.13);
  p3 += dot(p3, p3.yzx + 3.333);
  return fract((p3.x + p3.y) * p3.z);
}

float perlin(vec2 x) {
  vec2 i = floor(x);
  vec2 f = fract(x);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

// 颗粒抖动（对齐 Android gradientNoise）
float gradientNoise(vec2 uv) {
  return fract(52.9829189 * fract(dot(uv, vec2(0.06711056, 0.00583715))));
}

void main() {
  vec2 vUv = gl_FragCoord.xy / u_resolution;

  // 音频响应缩放（对齐 Android）
  float zoom = 1.0 + 0.04 * clamp(u_musicLevel, 0.0, 1.0) + 0.10 * u_beat;
  vec2 center = vec2(0.5);
  vec2 uv = (vUv - center) / zoom + center;

  // Beat 抖动（对齐 Android）
  uv += (u_beat * u_beat) * 0.006 * vec2(sin(u_time * 60.0), cos(u_time * 54.0));

  // Perlin 噪声（对齐 Android uNoiseScale=1.5）
  float noiseValue = perlin(vUv * 1.5 + vec2(-u_time, -u_time));

  // 圆运动参数（对齐 Android uPointOffset=0.1, uPointRadiusMulti=1.0）
  float pointOffset = 0.1 + 0.02 * u_musicLevel + 0.05 * u_beat;
  float pointRadiusMulti = 1.0 + 0.05 * u_musicLevel + 0.12 * u_beat;

  // 4 个点的位置和半径（对齐 Android setPhoneDark 默认值）
  // uPoints = {x, y, radius} × 4
  vec3 points[4];
  points[0] = vec3(0.63, 0.50, 0.88);
  points[1] = vec3(0.69, 0.75, 0.80);
  points[2] = vec3(0.17, 0.66, 0.81);
  points[3] = vec3(0.14, 0.24, 0.72);

  vec4 colors[4];
  colors[0] = u_color0;
  colors[1] = u_color1;
  colors[2] = u_color2;
  colors[3] = u_color3;

  // 混色循环（严格对齐 Android smoothstep 圆混合）
  vec4 color = vec4(0.0);

  for (int i = 0; i < 4; i++) {
    vec4 pointColor = colors[i];
    pointColor.rgb *= pointColor.a;
    vec2 point = points[i].xy;
    float rad = points[i].z * pointRadiusMulti;

    point.x += sin(u_time + point.y) * pointOffset;
    point.y += cos(u_time + point.x) * pointOffset;

    float d = distance(uv, point);
    float pct = smoothstep(rad, 0.0, d);

    color.rgb = mix(color.rgb, pointColor.rgb, pct);
    color.a   = mix(color.a,   pointColor.a,   pct);
  }

  // Premultiplied alpha 反转（对齐 Android）
  color.rgb /= max(color.a, 1e-5);

  // HSV 饱和度增强 — 仅音频驱动时生效（对齐 Android 181 行）
  vec3 hsv = rgb2hsv(color.rgb);
  hsv.y = clamp(hsv.y + (0.12 * u_musicLevel + 0.30 * u_beat) * u_saturateOffset, 0.0, 1.0);
  color.rgb = hsv2rgb(hsv);

  // 亮度偏移 — 仅音频驱动时生效（对齐 Android 183 行）
  color.rgb += (0.05 * u_musicLevel + 0.14 * u_beat) * u_lightOffset;

  // 透明度 + 轻度呼吸（对齐 Android uAlphaMulti=1.0）
  color.a = clamp(color.a, 0.0, 1.0);
  float alphaMod = clamp(1.0 - 0.18 * u_musicLevel - 0.12 * u_beat, 0.55, 1.0);
  color.a *= alphaMod;

  // 颗粒抖动（对齐 Android gradientNoise，消除色带和摩尔纹）
  color.rgb += (10.0 / 255.0) * gradientNoise(gl_FragCoord.xy) - (5.0 / 255.0);

  gl_FragColor = vec4(clamp(color.rgb, 0.0, 1.0) * color.a, color.a);
}
