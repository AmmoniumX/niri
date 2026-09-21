precision highp float;

#if defined(DEBUG_FLAGS)
uniform float niri_tint;
#endif

uniform float niri_alpha;
uniform float niri_scale;

uniform vec2 niri_size;
varying vec2 niri_v_coords;

uniform float colorspace;
uniform float hue_interpolation;
uniform vec4 color_from;
uniform vec4 color_to;
uniform vec2 grad_offset;
uniform float grad_width;
uniform vec2 grad_vec;

uniform mat3 input_to_geo;
uniform vec2 geo_size;
uniform vec4 outer_radius;
uniform float border_width;
// Phase, strength, brightness, nominal width. Width zero selects the static path.
uniform vec4 rainbow_ripple;

vec4 premul_rect(vec4 color) {
    color.rgb *= color.a;
    return color;
}

vec4 premul_lch(vec4 color) {
    color.xy *= color.a;
    return color;
}

vec4 unpremul_rect(vec4 color) {
    if (color.a == 0.0)
        return color;

    color.rgb /= color.a;
    return color;
}

vec4 unpremul_lch(vec4 color) {
    if (color.a == 0.0)
        return color;

    color.xy /= color.a;
    return color;
}

vec4 premul_mix_unpremul_rect(vec4 color1, vec4 color2, float ratio) {
    vec4 mixed = mix(premul_rect(color1), premul_rect(color2), ratio);
    return unpremul_rect(mixed);
}

vec4 premul_mix_unpremul_lch(vec4 color1, vec4 color2, float ratio) {
    vec4 mixed = mix(premul_lch(color1), premul_lch(color2), ratio);
    return unpremul_lch(mixed);
}

vec3 srgb_to_linear(vec3 color) {
    return pow(color, vec3(2.2));
}

vec3 linear_to_srgb(vec3 color) {
    return pow(color, vec3(1.0 / 2.2));
}

vec3 lab_to_lch(vec3 color) {
    float c = sqrt(pow(color.y, 2.0) + pow(color.z, 2.0));
    float h = degrees(atan(color.z, color.y)) ;
    h += h <= 0.0 ?
        360.0 :
        0.0 ;
    return vec3(
        color.x,
        c,
        h
    );
}

vec3 lch_to_lab(vec3 color) {
    float a = color.y * clamp(cos(radians(color.z)), -1.0, 1.0);
    float b = color.y * clamp(sin(radians(color.z)), -1.0, 1.0);
    return vec3(
        color.x,
        a,
        b
    );
}

vec3 linear_to_oklab(vec3 color){
    mat3 rgb_to_lms = mat3(
        vec3(0.4122214708, 0.5363325363, 0.0514459929),
        vec3(0.2119034982, 0.6806995451, 0.1073969566),
        vec3(0.0883024619, 0.2817188376, 0.6299787005)
    );
    mat3 lms_to_oklab = mat3(
        vec3(0.2104542553, 0.7936177850, -0.0040720468),
        vec3(1.9779984951, -2.4285922050, 0.4505937099),
        vec3(0.0259040371, 0.7827717662, -0.8086757660)
    );
    vec3 lms = color * rgb_to_lms;
    lms = pow(lms, vec3(1.0 / 3.0));
    return lms * lms_to_oklab;
}

vec3 oklab_to_linear(vec3 color){
    mat3 oklab_to_lms = mat3(
        vec3(1.0, 0.3963377774, 0.2158037573),
        vec3(1.0, -0.1055613458, -0.0638541728),
        vec3(1.0, -0.0894841775, -1.2914855480)
    );
    mat3 lms_to_rgb = mat3(
        vec3(4.0767416621, -3.3077115913, 0.2309699292),
        vec3(-1.2684380046, 2.6097574011, -0.3413193965),
        vec3(-0.0041960863, -0.7034186147, 1.7076147010)
    );
    vec3 lms = color * oklab_to_lms;
    lms = pow(lms, vec3(3.0));
    return lms * lms_to_rgb;
}

vec4 color_mix(vec4 color1, vec4 color2, float color_ratio) {
    vec4 color_out;

    // srgb
    if (colorspace == 0.0) {
        return mix(premul_rect(color1), premul_rect(color2), color_ratio);
    }

    color1.rgb = srgb_to_linear(color1.rgb);
    color2.rgb = srgb_to_linear(color2.rgb);

    // srgb-linear
    if (colorspace == 1.0) {
        color_out = premul_mix_unpremul_rect(color1, color2, color_ratio);
    // oklab
    } else if (colorspace == 2.0) {
        color1.xyz = linear_to_oklab(color1.rgb);
        color2.xyz = linear_to_oklab(color2.rgb);
        color_out = premul_mix_unpremul_rect(color1, color2, color_ratio);
        color_out.rgb = oklab_to_linear(color_out.xyz);
    // oklch
    } else if (colorspace == 3.0) {
        color1.xyz = lab_to_lch(linear_to_oklab(color1.rgb));
        color2.xyz = lab_to_lch(linear_to_oklab(color2.rgb));
        color_out = premul_mix_unpremul_lch(color1, color2, color_ratio);

        float min_hue = min(color1.z, color2.z);
        float max_hue = max(color1.z, color2.z);
        float path_direct_distance = (max_hue - min_hue) * color_ratio;
        float path_mod_distance = (360.0 - max_hue + min_hue) * color_ratio;

        float path_mod =
            color1.z == min_hue ?
                mod(color1.z - path_mod_distance, 360.0) :
                mod(color1.z + path_mod_distance, 360.0) ;
        float path_direct =
            color1.z == min_hue ?
                color1.z + path_direct_distance :
                color1.z - path_direct_distance ;

        // shorter
        if (hue_interpolation == 0.0) {
            color_out.z =
                max_hue - min_hue > 360.0 - max_hue + min_hue ?
                    path_mod :
                    path_direct ;
        // longer
        } else if (hue_interpolation == 1.0) {
            color_out.z =
                max_hue - min_hue <= 360.0 - max_hue + min_hue ?
                    path_mod :
                    path_direct ;
        // increasing
        } else if (hue_interpolation == 2.0) {
            color_out.z =
                color1.z > color2.z ?
                    path_mod :
                    path_direct ;
        // decreasing
        } else if (hue_interpolation == 3.0) {
            color_out.z =
                color1.z <= color2.z ?
                    path_mod :
                    path_direct ;
        }
        color_out.rgb = clamp(oklab_to_linear(lch_to_lab(color_out.xyz)), 0.0, 1.0);
    }

    return premul_rect(vec4(linear_to_srgb(color_out.rgb), color_out.a));
}

vec4 gradient_color(vec2 coords) {
    coords = coords + grad_offset;

    if ((grad_vec.x < 0.0 && 0.0 <= grad_vec.y) || (0.0 <= grad_vec.x && grad_vec.y < 0.0))
        coords.x -= grad_width;

    float frac = dot(coords, grad_vec) / dot(grad_vec, grad_vec);

    if (grad_vec.y < 0.0)
        frac += 1.0;

    frac = clamp(frac, 0.0, 1.0);
    return color_mix(color_from, color_to, frac);
}

float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

// Signed distance supports both the straight edges and individually rounded corners.
float rounded_rect_distance(vec2 coords, vec2 size, vec4 radii) {
    vec2 p = coords - size * 0.5;
    float radius = p.y < 0.0
        ? (p.x < 0.0 ? radii.x : radii.y)
        : (p.x < 0.0 ? radii.w : radii.z);
    radius = min(radius, min(size.x, size.y) * 0.5);
    vec2 q = abs(p) - size * 0.5 + radius;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

// Uneven periodic fields rather than equally spaced waves. Warping the sampling
// coordinate makes the lobes bunch up and stretch; all frequencies stay integral
// so the perimeter seam and the four-second animation loop remain continuous.
float wax_field(float u, float phase) {
    const float tau = 6.28318530718;
    return 0.46 * sin(tau * (3.0 * u - phase) + 0.7)
        + 0.28 * sin(tau * (7.0 * u + 2.0 * phase) + 2.1)
        + 0.17 * sin(tau * (11.0 * u - 2.0 * phase) + 4.4)
        + 0.09 * sin(tau * (19.0 * u + 3.0 * phase) + 1.3);
}

vec4 rainbow_color(vec2 coords) {
    const float tau = 6.28318530718;
    float phase = rainbow_ripple.x;
    float strength = rainbow_ripple.y;
    float width = rainbow_ripple.w;
    float padding = border_width - width;
    vec2 nominal_size = geo_size - vec2(padding * 2.0);
    vec2 p = (coords - geo_size * 0.5) / max(nominal_size * 0.5, vec2(1.0));
    float u = atan(p.y, p.x) / tau;
    float drift = u + 0.045 * wax_field(u + 0.13, phase);
    float bend = wax_field(drift, phase);
    float fine = wax_field(drift * 2.0 + 0.31, phase + 0.21);
    float pool = smoothstep(-0.65, 0.65, wax_field(drift + 0.43, phase + 0.37));

    // Both contours wander. Keep the inner edge outside the client, while the outer
    // one swells into broad pools joined by thin necks. Max outward reach is
    // width * (1 + 2.07 * strength), within FocusRing's reserved envelope.
    float inner_edge = width * strength * max(0.0, 0.40 + 0.34 * bend + 0.20 * fine);
    float thickness = width * mix(1.0, 0.28 + 1.65 * pool + 0.20 * fine, strength);
    float outer_edge = inner_edge + thickness;
    vec2 inner_size = geo_size - vec2(border_width * 2.0);
    vec4 inner_radius = max(outer_radius - vec4(border_width), 0.0);
    float distance = rounded_rect_distance(coords - border_width, inner_size, inner_radius);
    float half_px = 0.5 / niri_scale;
    float coverage = smoothstep(inner_edge - half_px, inner_edge + half_px, distance)
        * (1.0 - smoothstep(outer_edge - half_px, outer_edge + half_px, distance));

    // Swirled pastel pigment and narrow, broken highlights give the pools a waxy
    // surface. The highlight meanders across the band instead of whitening its
    // entire cross-section like a light travelling through a tube.
    float across = clamp((distance - inner_edge) / max(thickness, 0.01), 0.0, 1.0);
    float pigment = u - phase + 0.10 * bend + 0.025 * fine
        + 0.025 * sin(tau * (across * 0.65 + drift * 8.0 + phase));
    vec3 hue = clamp(abs(fract(pigment + vec3(0.0, 2.0/3.0, 1.0/3.0)) * 6.0 - 3.0) - 1.0, 0.0, 1.0);
    float milk = clamp(0.30 + 0.18 * bend + 0.12 * pool, 0.12, 0.62);
    vec3 rgb = mix(hue, vec3(1.0), milk);
    float ridge = 0.48 + 0.18 * fine;
    float sheen = exp(-pow((across - ridge) / 0.17, 2.0))
        * smoothstep(0.30, 0.85, pool) * (0.65 + 0.35 * bend);
    rgb = mix(rgb, vec3(1.0), 0.8 * sheen);
    rgb *= 0.90 + 0.10 * sin(tau * (across * 0.5 + 0.1 * bend));
    rgb = clamp(rgb * rainbow_ripple.z, 0.0, 1.0);
    float alpha = coverage * gradient_color(coords).a;
    return vec4(rgb * alpha, alpha);
}

void main() {
    vec3 coords_geo = input_to_geo * vec3(niri_v_coords, 1.0);
    vec4 color;
    if (rainbow_ripple.w > 0.0) {
        color = rainbow_color(coords_geo.xy);
    } else {
        color = gradient_color(coords_geo.xy);
        color = color * niri_rounding_alpha(coords_geo.xy, geo_size, outer_radius);

        if (border_width > 0.0) {
            coords_geo -= vec3(border_width);
            vec2 inner_geo_size = geo_size - vec2(border_width * 2.0);
            if (0.0 <= coords_geo.x && coords_geo.x <= inner_geo_size.x
                    && 0.0 <= coords_geo.y && coords_geo.y <= inner_geo_size.y)
            {
                vec4 inner_radius = max(outer_radius - vec4(border_width), 0.0);
                color = color * (1.0 - niri_rounding_alpha(coords_geo.xy, inner_geo_size, inner_radius));
            }
        }

    }

    color = color * niri_alpha;

#if defined(DEBUG_FLAGS)
    if (niri_tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
