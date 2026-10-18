# Touchcraft icon

The app icon uses Phosphor's regular **Graph** glyph on a deep periwinkle
(`#6554d9`) rounded tile, with warm paper (`#efe9dc`) line work. The tile follows
the Craft apps' macOS proportions: an 824 px body centered on a transparent
1024 px canvas, with a 112/512 corner-radius ratio.

`touchcraft-icon.svg` is the vector master. `touchcraft-icon.png` is its 1024 px
render, and `touchcraft.icns` contains macOS icon sizes. Regenerate them with
`python3 assets/generate-icon.py` from the repository root; ImageMagick and
macOS `iconutil` are required.

The glyph is from [Phosphor Icons](https://github.com/phosphor-icons/core/blob/main/assets/regular/graph.svg),
copyright 2023 Phosphor Icons, under the MIT license in `PHOSPHOR-LICENSE.txt`.
The original vector is preserved as `phosphor-graph.svg`.

`EGUI-PHOSPHOR-LICENSE.txt` preserves the MIT license for the application's
egui-phosphor font integration, copyright 2023 Romet Tagobert.
