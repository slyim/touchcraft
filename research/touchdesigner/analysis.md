# TouchDesigner reference analysis

Inspection date: 2026-10-18. Scope: read-only inspection of the installed macOS bundle using REA 6.7.0, plus public Derivative documentation. Touchcraft is an original Rust/egui implementation of a bounded image-operator workflow.

## Verified installed-artifact evidence

| Evidence file | REA evidence ID | Observed facts |
| --- | --- | --- |
| [macho.json](macho.json) | `ev_72f1a7f4745bacc160adaa853a44c29bfe82787129212920c8870e29b5c8832d` | The executable is a little-endian, 64-bit ARM64 Mach-O. Build metadata specifies macOS 13.0 minimum and SDK 26.2. Its `__text` section is 188 bytes. The dependency list contains 117 load commands, including `libTOP`, `libCHOP`, `libSOP`, `libDAT`, `libMAT`, `libOP`, `libPRM`, `libOPUI`, `libTER`, and `libTouchEngine`. |
| [plist.json](plist.json) | `ev_fd9134907b8e2e48d84f89901c6f5ae0285c0d7e5b5d5a76e008e7b2480332de` | The bundle identifies itself as `ca.derivative.TouchDesigner`, version `2025.33070`, and declares `.toe` environment and `.tox` component documents. Camera and microphone usage descriptions reference Video Device In TOP and Audio Device In CHOP. |
| [core-macho.json](core-macho.json) | `ev_5a1e376b9c7d735fbb0cf3bce0044ed0c6d88e82ec897eb0454f894fc650f11f` | A focused follow-up inspects `Contents/Frameworks/libTER.dylib`: ARM64, macOS 13.0 minimum, 304,144 bytes in `__text`, and 41 dependencies. It exports the `TER_TouchExeBase` construction, `runMain`, and destruction symbols imported by the launcher. Its dependencies include MoltenVK, Python 3.11, OpenColorIO 2.5, `libTOP`, `libOP`, `libOPUI`, and `libPRM`. |

The launcher and plist envelopes identify `/Applications/TouchDesigner.app/Contents/MacOS/TouchDesigner` with SHA-256:

```text
29ba7e038cc1b101df4e5e4ab8c8d5148961680a5dc6240040dd157f9343e669
```

The plist envelope's subject digest is the executable digest above; it is not an independently reported digest of `Info.plist`. Its actual inspected source is recorded as `/Applications/TouchDesigner.app/Contents/Info.plist`.

The executable imports `TER_TouchExeBase` construction, `runMain`, and destruction from `libTER`. Combined with its small code section, this supports the inference that the executable primarily delegates startup to shared libraries. Dependency names indicate modular organization; they do not establish the libraries' internal algorithms or complete responsibilities.

The focused library envelope identifies `/Applications/TouchDesigner.app/Contents/Frameworks/libTER.dylib` with SHA-256:

```text
3b9c7880e2ddfafa5d352238648ea5808b338e67f7551615c59ecda03737bba0
```

This follow-up confirms a concrete launcher-to-library symbol relationship. The filename `core-macho.json` names the saved follow-up, not proof that `libTER` contains the complete processing engine. Its graphics and scripting dependencies establish linkage only; no particular rendering backend behavior or Python API implementation was recovered.

The local REA diagnostic reports CLI version `6.7.0` and Node `26.12.0`. Its explicit Codex scope is unhealthy because the Codex MCP registration is missing (`config_drift`). It also reports unavailable Hopper, Ghidra, IDA, jadx, and Kuna analysis engines. This does not invalidate the successful native `inspect_macho` and `inspect_plist` results. No registration or provider installation was performed. The diagnostic stays local because it includes machine configuration details unrelated to the target.

## Documented workflow, separately sourced

- [Operator families](https://docs.derivative.ca/Operator_Family): generators create data; filters transform inputs; direct wires connect within a family. CHOP exports can control parameters across families.
- [Operator creation](https://docs.derivative.ca/OP_Create_Dialog): Tab, background double-click, or the network's + control opens searchable operator creation.
- [Parameters](https://docs.derivative.ca/Parameter) and [panes](https://docs.derivative.ca/Pane): selected-node controls, a network editor, output viewers, and a bottom timeline provide the main workflow.
- [TOP](https://docs.derivative.ca/TOP), [Noise](https://docs.derivative.ca/Noise_TOP), [Transform](https://docs.derivative.ca/Transform_TOP), and [Composite](https://docs.derivative.ca/Composite_TOP): image generation, transformation, and blending form a useful connected pipeline.
- [LFO CHOP](https://docs.derivative.ca/LFO_CHOP): realtime signals can provide numeric modulation. [Getting started](https://docs.derivative.ca/Getting_started) documents exporting channels to operator parameters.

These are public behavior references, not conclusions derived from binary metadata. Direct documentation fetches returned HTTP 403; indexed official documentation excerpts were accessible.

## Touchcraft implementation boundary

Touchcraft targets a local TOP-style graph editor: create and connect supported image nodes, inspect generated output, edit parameters, control playback, and save/reload its own project format. Phosphor supplies interface icons. Its graph evaluation and image generation are original code; no Derivative binaries, shaders, icons, or source are incorporated.

No claim is made that Touchcraft reproduces TouchDesigner's GPU cooking engine, numerical output, full operator catalog, scripting, hardware integrations, or `.toe`/`.tox` compatibility. Those behaviors and file formats were not established by this inspection.

## Runtime observation status

A targeted `pgrep -x TouchDesigner` check could not access the process list: `sysmon request failed with error: sysmond service not found`; exit status 3. Whether TouchDesigner was running therefore remains unverified. No target launch, UI interaction, or settings changes were performed, and no runtime layout screenshot was obtained. Documented workflow remains the behavior reference.

Remaining limitations: metadata and documented workflow evidence do not recover implementation semantics. Native artifact symbol listings can omit stripped or toolchain-hidden symbols. Operator parity and compatibility require separate behavioral fixtures and testing.
