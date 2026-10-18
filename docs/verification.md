# Verification — 18 October 2026

The release bundle is `dist/Touchcraft.app`. The application was built locally for Apple Silicon macOS with Rust 1.99.1, eframe/egui 0.36.2, and egui-phosphor 0.14.0.

- `cargo test --offline`: 10 tests passed. These cover graph cycles, malformed projects, allocation boundaries, serialization, animated and parameter-dependent pixels, atomic save preservation, undo versus rename buffers, keyboard isolation during unsaved-change prompts, native close decisions, and network fit at supported window sizes.
- `cargo clippy --offline --all-targets -- -D warnings`: passed.
- The Impeccable mechanical detector returned no findings for the Rust UI sources; its coverage does not replace native visual inspection.
- macOS bundle plist and packaging script syntax checks passed. Ad-hoc signing verification passed with `codesign --verify --deep --strict`.
- The packaged executable rendered successfully and exited after screenshots at 1480×900 and 1100×720 logical window sizes. Physical framebuffer captures are in `target/touchcraft-desktop.png` and `target/touchcraft-compact.png`. Both layouts were visually inspected. Initial fit now responds to canvas dimension changes; all five starter nodes remain visible in the compact layout.

The first sandboxed native launch could not access macOS window services. It was stopped; native verification succeeded through approved execution outside the sandbox. This was an environment-access failure, separate from application rendering.

REA 6.7.0 successfully inspected the TouchDesigner executable, bundle plist, and startup library. Findings, evidence IDs, digests, and limits are recorded in `research/touchdesigner/analysis.md`. No native decompiler is installed/configured. The target's runtime state could not be checked inside the restricted process-list environment; no reference-app runtime behavior is claimed.

Native file-dialog clicks and physical mouse/keyboard gestures were not automated. Input-state simulation covers document Delete/Undo and modal keyboard isolation; model tests cover save/load and graph operations. Full TouchDesigner parity, GPU throughput, `.toe`/`.tox` compatibility, and device/media integration remain outside this first implementation.
