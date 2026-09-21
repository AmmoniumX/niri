precision highp float;
uniform float niri_alpha;
uniform float light_gain;
uniform vec2 light_tex_scale;
uniform sampler2D light_texture;
varying vec2 niri_v_coords;
#if defined(DEBUG_FLAGS)
uniform float niri_tint;
#endif

void main() {
    vec3 light = texture2D(light_texture, niri_v_coords * light_tex_scale).rgb;
    // Soft exposure preserves colour while bounding the screen-blend contribution.
    light = (vec3(1.0) - exp(-light * light_gain)) * niri_alpha;
    gl_FragColor = vec4(light, max(light.r, max(light.g, light.b)));
}
