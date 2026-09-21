# Focus-ring and border shaders

## Configuration or shader implementation?

The existing effect is configured with:

```kdl
layout {
    focus-ring {
        on
        width 6
        rainbow-ripple speed=1.0 strength=0.75 brightness=1.0
    }
}
shader-animation-max-fps 60
```

`resources/shaders/focus-ring/rainbow-ripple.kdl` is a ready-to-include preset. The FPS cap is an optional top-level setting shared by animated shaders. The same effect settings work in `border` and in window-rule `focus-ring` / `border` blocks.

Use configuration for tuning the existing effect. New rendering algorithms require changing the built-in shader and rebuilding biri: `focus-ring` currently has no arbitrary GLSL `source` or `path` option. Editing its embedded `.frag` file does not hot-reload the running compositor.

Defaults and supported ranges are defined by `RainbowRipple` in `niri-config/src/appearance.rs`: `enable=true`, `speed=1.0` (0–10), `strength=0.75` (0–1), and `brightness=1.0` (0–2). A speed of zero freezes the effect; zero strength keeps its contours still while colours can move. A rule replaces the complete effect settings, with omitted properties taking defaults. `rainbow-ripple enable=false` restores the configured colours/gradients.

See `docs/wiki/Configuration:-Layout.md`, under “Rainbow ripple”, for the user-facing contract. Keep the preset, defaults, and documentation aligned when changing them.

## Where to implement an effect

| Concern | Repository file |
| --- | --- |
| Colour, silhouette, antialiasing, premultiplied output | `src/render_helpers/shaders/border.frag` |
| GLSL uniform registration and compilation | `src/render_helpers/shaders/mod.rs` |
| Rust uniform values and render-element damage | `src/render_helpers/border.rs` |
| Ring pieces, bounds, effect selection, phase | `src/layout/focus_ring.rs` |
| Clock input and fullscreen/animation policy | `src/layout/tile.rs` |
| Visible-decoration queries | `src/layout/monitor.rs`, `src/layout/mod.rs` |
| Idle redraw scheduling and shader FPS cap | `src/niri.rs` |
| Config defaults, parsing, rule merging | `niri-config/src/appearance.rs` |

Focus rings and ordinary borders share this renderer. Preserve the static colour/gradient path and the disabled-effect uniform value so changes do not animate other decorations accidentally. Adding a uniform requires updating its GLSL declaration, registration, and Rust value together.

## Coordinates and animation contract

The shader is GLES2 GLSL ES 1.00. The compiler supplies `#version 100`; the built-in file has its own `main()` and writes `gl_FragColor`. It does not use the global shader's `global_color`, `tex2D_screen`, cursor, or injected time contract.

The hollow ring renders as eight pieces: four edges and four corners. Compute shape and colour from their shared geometry so there are no seams at piece boundaries.

| Value | Meaning |
| --- | --- |
| `niri_v_coords` | Normalized coordinates within the current piece |
| `input_to_geo * vec3(niri_v_coords, 1.0)` | Coordinates in the full decoration's logical-pixel geometry, with Y down |
| `geo_size`, `outer_radius` | Full padded decoration size and corner radii in logical pixels |
| `border_width` | Reserved width, including the animation envelope; not necessarily the configured width |
| `niri_size` | Current draw destination size in physical pixels; not the full window size |
| `niri_scale` | Output scale, used to antialias logical distances |
| `rainbow_ripple` | `vec4(phase, strength, brightness, nominal_width)`; width zero disables the effect |

Rust derives phase from the layout clock as `(seconds * speed / 4).rem_euclid(1)` in f64 before conversion to f32. Keep procedural fields periodic across phase 0/1 and across the perimeter seam. Integer harmonics and periodic coordinate warps are one way to do this; arbitrary time-noise will pop at the four-second wrap unless it is made periodic too.

The effect returns premultiplied RGB and alpha. Preserve the active colour/gradient's opacity, and let the existing `main()` apply `niri_alpha` once. Use an antialias interval tied to `niri_scale`, such as half a physical pixel (`0.5 / niri_scale`).

## Shape, material, and bounds

When matching a video, inspect frames at several times and compare the silhouette, thickness, colour motion, and highlights separately. Cycling rainbow colours on a largely fixed outline can resemble a fluorescent tube even when it is animated.

For the wax-like reference, the useful changes were independently wandering inner and outer contours, broad uneven pools joined by thinner necks, warped periodic fields instead of equally spaced waves, softer pigment, and narrow broken highlights that drift across the band. Preserve the requested aesthetic for other references rather than applying this recipe universally.

Keep the animated ring hollow and outside the client cutout, including with transparent windows. Varying the inner contour can move it away from the client without painting over client content. Do not change window layout sizes to accommodate bulges.

Reserve a fixed envelope for the maximum possible deformation, including antialiasing, and use it in all eight pieces. In the current wax shader the maximum outward reach is bounded by `width * (1 + 2.07 * strength)`; Rust reserves `width * 2.2 * strength` plus one physical pixel beyond the nominal width, rounded up to physical pixels. If the displacement formula changes, recompute this bound and update `FocusRing`'s padding. Visibility checks use `render_outset()` so a bulge crossing the output edge can still animate.

## Redraw behaviour to preserve

Changing phase must update the render element's commit/damage state. Passing a time value to the GPU alone will not repaint an otherwise idle desktop. Keep decoration animation in the shader scheduling path so `shader-animation-max-fps` applies, rather than classifying it as a continuously running layout transition that bypasses the cap.

Preserve the existing policy: active decorations use the effect; inactive and urgent ones retain configured colours/gradients. Disabled, zero-width, transparent, fully expanded fullscreen/maximized, and hidden decorations should not independently request frames. Account for sticky windows, interactive moves, and overview geometry in visibility queries. The layout clock's disabled-animation mode makes the effect static, and a frozen clock or zero effect speed needs no animation wakeups. The output scheduler also suppresses decoration-driven wakeups during lock and screenshot UI states.

## Preview using the actual shader

Run from the repository root in its dev shell. The EGL test creates a surfaceless renderer and initializes both render resources and shaders; omitting resource initialization can produce completely transparent frames even when GLSL compiles.

The current test can export 120 frames covering one four-second cycle at 30 FPS:

```sh
NIRI_RAINBOW_PREVIEW_DIR=/tmp/biri-focus-preview direnv exec . cargo test --offline -p niri --lib egl_rainbow_ripple_pixels
ffmpeg -v error -framerate 30 -i /tmp/biri-focus-preview/frame-%03d.png -vf 'scale=768:488' -c:v libx264 -pix_fmt yuv420p -movflags +faststart -y /tmp/biri-focus-preview.mp4
```

The fixture currently uses width 8, strength 0.75, and mixed corner radii. Adjust or extend the fixture when previewing another effect or parameter set; do not label these frames as a capture of the user's live desktop. Inspect rendered frames and the resulting clip before sharing. Use a new output path for a revision when side-by-side comparison is useful.

Validate observable properties relevant to the change: visible pixels, changing frames, loop continuity, hollow centre, unclipped bounds, and normal/fractional/double scale. For the wax effect, the GPU test also checks variation in both contours and thickness, plus a steady silhouette at zero strength. Config/rule merging and hidden/fullscreen/frozen redraw behaviour have separate tests.

When changing shared config structs or rendering APIs, include `niri-visual-tests` in compilation checks: its explicit decoration initializers can break even when compositor/config tests pass. Workspace-wide checks need GTK4 and libadwaita development libraries. The repository's `nix develop .` shell provides these; a compositor-only local dev shell may not.

Useful checks and build commands (use `nix develop . --command` in place of `direnv exec .` when the local shell lacks those libraries):

```sh
direnv exec . cargo test --offline -p niri --lib layout::
direnv exec . cargo test --offline -p niri-config
direnv exec . cargo fmt --all --check
direnv exec . cargo clippy --offline --workspace --all-targets
direnv exec . cargo build --offline -p niri --bin niri
direnv exec . target/debug/niri validate --config resources/shaders/focus-ring/rainbow-ripple.kdl
```

For wiki edits, also run `uv run --locked mkdocs build --strict` from `docs/`. Report the checks actually completed and any failures separately. A successful offscreen preview verifies shader rendering, not live-desktop performance or every capture backend.
