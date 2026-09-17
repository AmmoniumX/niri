# Vertical scrolling for portrait outputs

## Scope and research

The Reddit wishlist asks for infinite vertical scrolling on portrait monitors, configurable
per output or workspace, alongside normal horizontal scrolling on landscape monitors.
The Biri board was read on 2026-09-17: no vertical-layout item exists. This implements the
previously deferred request; it does not change the board or post upstream comments.

References read:

- [niri #1071](https://github.com/niri-wm/niri/issues/1071), including its comments: actual
  vertical scrolling, not simply a shorter default window or automatic stacking.
- [Portrait-mode PR #1677](https://github.com/niri-wm/niri/pull/1677): early draft using
  typed u/v coordinates. Review asks for descriptive axis names and physical directional
  shortcuts. No patch taken from this draft.
- [WorkspaceDimensions PR #1690](https://github.com/niri-wm/niri/pull/1690): preparatory
  refactor, not a feature implementation. No patch taken from this draft.
- [Discussion #4585](https://github.com/niri-wm/niri/discussions/4585): per-output default
  window height as an interim workaround; it does not provide a vertical strip.
- [0WD0's reference branch](https://github.com/0WD0/niri/tree/5b981892b944675821bca6e561dc6588df4b92f7):
  the implementation base, spanning commits 678944ae through 5b981892. The author describes it as a reference,
  not a reviewed upstream contribution.

## Design

`layout { main-axis "vertical" }` uses the existing layout override hierarchy: workspace,
then output, then global. The default remains horizontal. Scrolling and column internals
use main/cross coordinates; tiles, clients, rendering, IPC positions and input devices
use physical coordinates. `AxisMap` defines the boundary between them. Windows stay upright.

Column-width presets set the main span; window-height presets set the cross span. Basic
focus/move directions stay physical, following the upstream maintainer's review. Explicit
column operations (indices, first/last, consume/expel) keep their layout-relative meaning.
Workspaces stack across the scrolling axis in the overview. Floating placement stays physical.

## Changes beyond the reference branch

- Resolve Biri's multi-action binds and preserve Shift+wheel carousel rotation.
- Preserve sticky-window drag handling and hidden-workspace filtering and indexing.
- Share live and forced-zoom carousel geometry and crop previews along the correct axis.
- Use the carousel lens target's layout to interpret wheel input.
- Fix the reference's stale `column_pos` identifier in tab hit testing.
- Keep struts, client bounds, resize-edge metadata, closing-window size, tile render view
  rectangles and tab indicators in physical coordinates at the boundary.
- Convert animation offsets on floating/tiling transitions, workspace moves and drag/drop.
- Apply initial min/max constraints along the appropriate scrolling axes.
- Extend randomized layout options to include both axes and orientation changes.

## Verification

Regression coverage includes vertical placement, insert targets, edge scrolling, overview,
physical navigation, resizing, dragging, floating sizing, asymmetric struts, client configure
sizes/bounds, output/workspace override precedence, reload, migration and carousel geometry.
Existing layout, client/server, config and IPC suites are also run. Physical-monitor,
touchpad and visual carousel testing remain manual checks; headless tests cannot establish
that those interactions look and feel correct on hardware.

Validation results (2026-09-17):

- Default-feature `cargo check` and `cargo clippy --all-targets` pass; Clippy reports
  existing warnings in the fork.
- Full suite with `XDG_SEAT=biri-test LIBGL_ALWAYS_SOFTWARE=1 cargo test --all
  --exclude niri-visual-tests`: 336 compositor tests, 75 config unit tests, config's
  documentation parser, 3 IPC tests and the IPC doctest pass.
- 17 focused vertical-layout regressions pass, including the two added after that full run.
  The detached drag-anchor test was first confirmed failing and then passed after the fix.
- 2,000 randomized layout cases pass, including axis changes in global/output/workspace
  settings. The interactive move anchor fix was subsequently checked by the focused tests.
- `cargo fmt --all -- --check`, `git diff --check`, and strict MkDocs build pass.
- After the reuse/comment cleanup: 197 layout tests and 3 input-axis tests pass; formatting,
  Clippy and `cargo build --bin niri` pass. Screenshot controls reuse their original API,
  and input geometry delegates to the shared `AxisMap`.
- Tests use an empty input seat because the headless backend otherwise enumerates real
  input devices. The GTK visual-test application and a live portrait-monitor session were
  not run.
