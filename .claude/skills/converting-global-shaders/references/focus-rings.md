# Focus-ring and border shaders

## Use external files for new effects

Create a `.frag` file and point a decoration's `shader` block at it. Do not add a hard-coded Rust/GLSL effect or require rebuilding the compositor to author another ring.

```kdl
window-rule {
    match app-id=r#"^com\.mitchellh\.ghostty$"#
    focus-ring {
        on
        width 6
        shader {
            path "~/.config/niri/focus-ring/rainbow-ripple.frag"
            padding 14
        }
    }
}
```

Use `~/.config/niri/config.kdl` and `~/.config/niri/focus-ring/` for standard-install examples; the fork retains the `niri` and `niri-session` command names. Honour an existing custom config location when changing a live setup. Inspect existing files before copying bundled examples, and do not overwrite user-authored effects. `rainbow-ripple.frag` is the waxy rainbow; `pulse.frag` is a simple independent cyan pulse; `lightning.frag` has a travelling blue-white crackle. The `.kdl` preset sets a global rainbow ring and resolves its `.frag` beside itself.

The same block works in `layout`, output/workspace layouts, `border`, and per-window rules. Each application may name a different file. `source "..."` is an inline alternative, mutually exclusive with `path`. Relative paths resolve beside the containing config/include; `~` expands to home.

Shader files join the config watcher's dependencies and reload when saved (500 ms polling); unchanged KDL does not prevent source changes from compiling. `niri msg action load-config-file` also forces a load. Missing files and invalid GLSL log diagnostics and fall back to configured colours; fixing the file restores the shader. Test this rather than promising that a compile failure preserves the previous effect.

A rule replaces the complete shader block. Defaults: `enable true`, `animated true`, `speed 1` (0–10), `padding 0` (0–1024 logical pixels). `animated false` or `speed 0` freezes time at zero and avoids animation wakeups. `shader { enable false; }` disables an inherited effect. An explicit shader block takes precedence over legacy `rainbow-ripple`, even if disabled or invalid.

## GLSL contract

Write GLES2 / GLSL ES 1.00 without `#version` or `main()`:

```glsl
vec4 ring_color(vec2 coords) {
    float d = ring_distance(coords);
    float half_px = 0.5 / niri_scale;
    float coverage = smoothstep(-half_px, half_px, d)
        * (1.0 - smoothstep(ring_width - half_px, ring_width + half_px, d));
    return vec4(0.2, 0.8, 1.0, coverage);
}
```

Return **straight RGBA**. The wrapper clamps alpha, multiplies configured colour/gradient opacity, masks the client cutout, premultiplies RGB, and applies window opacity once. Shape and antialias the outer boundary yourself. Do not multiply opacity again. There is no window/screen texture input.

| Symbol | Meaning |
| --- | --- |
| `coords` | Logical pixels relative to client top-left, X right/Y down; negative outside |
| `ring_size` | Client dimensions in logical pixels |
| `ring_width` | Nominal configured width |
| `ring_padding` | Extra outward envelope, including output-pixel rounding and an AA margin |
| `ring_radius` | Client corner radii: TL, TR, BR, BL |
| `ring_distance(coords)` | Signed distance from rounded client edge, positive outside |
| `ring_base_color(coords)` | Configured gradient/colour, straight RGBA |
| `niri_time` | Layout-clock seconds multiplied by speed; zero for static shaders |
| `niri_scale` | Output scale; use `0.5 / niri_scale` for AA half-width |

All eight ring pieces share these coordinates. Do not build effects using `niri_v_coords` or `niri_size`, which describe the individual piece. Global shaders' samplers, coordinate contract, and capture restrictions do not apply: rings use ordinary decoration rendering on TTY, nested, and headless backends.

## Shape and material

When matching a reference, compare silhouette, thickness, colour motion, and highlights at several times. Colour cycling on a fixed outline can look like a fluorescent tube.

For wax, use independently wandering contours, uneven pools joined by thin necks, warped periodic fields, pastel pigment, and broken highlights. The bundled shader uses a four-second periodic phase and integer spatial/temporal harmonics to avoid seams. Its `WAX_STRENGTH` and `WAX_BRIGHTNESS` constants are editable; changing the algorithm is also just a file edit.

Reserve sufficient `padding` for the maximum outward deformation without changing layout sizes. The wax effect needs at least `width * 2.2 * strength` extra space (the compositor adds an AA pixel). At width 6–8 and strength 0.75, padding 14 suffices. If widening the ring or increasing strength, increase padding too. Pixels beyond the reserved rectangle are clipped. The client stays hollow even if custom code returns an opaque colour everywhere.

## Light spill from existing shaders

Inspect the user's existing shader files before creating a new effect. For light following a bright pulse, keep `ring_color` unchanged and add this child inside its existing `shader` block:

```kdl
light spread=80 intensity=1.0 threshold=0.5
```

Lighting is opt-in. `enable=false` or `intensity=0` disables it; spread is 1–256 logical pixels (soft tails extend further), intensity 0–4, threshold 0–1. Defaults match the example. The cutoff uses the brightest premultiplied RGB channel after ring opacity, so increase it to isolate bright pulses or lower it for dim shaders. Existing path watching and config reload handle future changes without rebuilding. Keep the user's effect file and other rules intact.

The compositor renders the actual ring into a half-resolution emission texture at exactly the same time, extracts bright pixels, applies a cached Dual Kawase blur, then screen-blends it over scene windows. This can illuminate its own client and neighbouring/floating/sticky windows; it is screen-space bloom, not ray tracing or material-aware lighting. The ring cutout remains hollow. No screen sampler is exposed to custom GLSL. `padding` reserves ring geometry; lighting has its own expanded bounds. Opening transforms and closing snapshots omit the separate scene light; output capture includes it, isolated window capture does not.

Preserve light ordering above all window content and below compositor/shell overlays, including overview transforms. Track damage over the full halo when phase, geometry, opacity, shader or lighting settings change. Static emission must reuse the blur; disabled/missing/invalid shaders must fall back without lighting. Prefer float intermediate textures to avoid amplified 8-bit banding, with compatible fallbacks. Include halo bounds in visible animation checks.

## Implementation map (only for changing the shader infrastructure)

| Concern | File |
| --- | --- |
| User-editable effects | `resources/shaders/focus-ring/*.frag` |
| Config parsing, source loading, watcher dependencies, content keys | `niri-config/src/decoration_shader.rs` |
| Rule inheritance | `niri-config/src/appearance.rs` |
| Coordinate contract, client mask, normal-ring fallback | `src/render_helpers/shaders/border.frag` |
| Program cache, compilation and resource retirement | `src/render_helpers/shaders/mod.rs` |
| Uniforms, program changes and render damage | `src/render_helpers/border.rs`, `shader_element.rs` |
| Emission cache, blur and screen-blend composite | `src/render_helpers/decoration_light.rs`, `shaders/decoration_light.frag` |
| Light placement and stacking | `src/layout/tile.rs`, `monitor.rs` |
| Eight pieces, padding, phase, visible animation state | `src/layout/focus_ring.rs` |
| Startup on all backends and reload | `src/backend/{tty,winit,headless}.rs`, `src/niri.rs` |
| File edit detection | `src/utils/watcher.rs` |

The old `rainbow-ripple speed=1 strength=0.75 brightness=1` is a compatibility shorthand compiled from the same bundled `.frag`, not the workflow for new shaders. Keep its existing configurations working.

Preserve damage updates when time or programs change; time uniforms alone do not repaint idle windows. Keep decoration animation in the shader FPS-cap path. Disabled, zero-width, transparent, inactive, urgent, hidden, fullscreen/maximized, failed, and static shaders should not independently request frames. Respect the animation clock, overview/sticky/interactive-move visibility, lock, and screenshot UI states.

## Verification and previews

Use the compositor's actual EGL renderer with resources **and** shaders initialized. The file-based GPU test verifies independent per-window programs, reload from the same path, normal-colour fallback, recovery, cache retirement, static shaders, client cutouts, and pixel equivalence to the original wax at 1×/1.25×/2× scales.

```sh
direnv exec . cargo test --offline -p niri --lib decoration
direnv exec . cargo test --offline -p niri --lib layout::
direnv exec . cargo test --offline -p niri-config
direnv exec . cargo fmt --all --check
nix develop . --command cargo clippy --offline --workspace --all-targets
```

Workspace-wide checks must include `niri-visual-tests`: it has explicit decoration initializers and needs GTK4/libadwaita. A compositor-only dev shell may not provide those; the repository's Nix shell does.

For a wax preview, the regression fixture exports a four-second loop using the same shader source:

```sh
NIRI_RAINBOW_PREVIEW_DIR=/tmp/biri-focus-preview direnv exec . cargo test --offline -p niri --lib egl_rainbow_ripple_pixels
ffmpeg -v error -framerate 30 -i /tmp/biri-focus-preview/frame-%03d.png -vf 'scale=768:488' -c:v libx264 -pix_fmt yuv420p -movflags +faststart -y /tmp/biri-focus-preview.mp4
```

Extend the file-based fixture to preview another effect. Inspect rendered frames before sharing; label them as offscreen renders, not live desktop captures. Test visible output, movement, loop continuity, both contours, hollow centre, bounds, and fractional scale. Run `uv run --locked mkdocs build --strict` from `docs/` for wiki changes. Report checks actually completed and their limitations.

For a lightning spill preview using the real compositor renderer:

```sh
NIRI_LIGHT_PREVIEW_DIR=/tmp/biri-light-preview direnv exec . cargo test --offline -p niri --lib egl_decoration_light_spills_onto_window_content
ffmpeg -v error -framerate 30 -i /tmp/biri-light-preview/frame-%03d.png -c:v libx264 -pix_fmt yuv420p -movflags +faststart -y /tmp/biri-light-preview.mp4
```

The test checks illumination on both the owner and a neighbour, pulse motion, unchanged distant pixels, non-darkening blend, cache reuse, opacity damage and threshold reload at 1×/1.25×/2×. The layout test checks light ordering over floating and tiled decorations in normal view and overview, preserving ordinary stacking order when disabled. Inspect frames for banding as well as alignment.
