# Touchcraft

A local Rust/egui visual programming editor with Phosphor Icons, inspired by TouchDesigner's image-operator workflow. This is an independent first implementation, not a compatible TouchDesigner runtime.

## Run

```sh
cargo run --offline
```

Build a macOS application bundle:

```sh
./packaging/build-macos.sh
```

The resulting app is `dist/Touchcraft.app`. The bundle is locally ad-hoc signed, not notarized.

## Use

A connected starter network opens immediately. Select an operator to view its image and parameters. Add Noise, Circle, Constant, Transform, Level, Composite, or Output from the library or with Tab. Double-click empty network space to open searchable creation. Click or drag an output connector to an input connector; the inspector's Connections menus provide the same operation. Cycles are rejected.

Drag nodes to arrange the network. Middle-drag or Alt-drag pans; scroll zooms; Fit frames all operators. Space pauses/resumes animation. The transport slider scrubs time. Delete removes the selected operator; Command-Z and Shift-Command-Z undo and redo. Existing operators are selectable from the left list as well as the canvas.

Save/Open use original `.touchcraft` JSON projects. Saves validate first and replace files atomically. Failed opens retain the current graph. New/Open/Quit prompt for unsaved changes; cancelling a save keeps the current project open. On macOS, the shared Craft quit integration covers Command-Q and close requests while minimized.

## Verification

```sh
cargo test --offline
cargo run --offline -- --screenshot /tmp/touchcraft.png
cargo run --offline -- --compact --screenshot /tmp/touchcraft-compact.png
```

The screenshot commands open a real native window, capture the egui renderer, then quit. They are rendering checks, not physical keyboard or file-dialog automation. Graph tests cover cycles, dangling references, allocation bounds, rendering changes, serialization, and preservation of existing files on failed saves.

## Research and limits

[REA evidence and findings](research/touchdesigner/analysis.md) record the inspected TouchDesigner version, artifact hashes, exact evidence IDs, public workflow references, and uncertainties. REA 6.7.0 was used through the locally installed CLI; no agent settings or analysis providers were installed.

The preview engine is original CPU code. It has a 256-operator project limit and a one-megapixel render limit. Large connected graphs can be slower. This release implements only image operators; CHOP/SOP/DAT/MAT/COMP systems, GPU cooking, scripting, media input/output, devices, and `.toe`/`.tox` import are not implemented. No rendering parity is claimed.

Icon sources and MIT attribution are in [assets](assets/README.md). The macOS Quit helper is pinned to the same shared Craft library revision already used by PaperCraft.
