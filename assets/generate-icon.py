#!/usr/bin/env python3
"""Render the Phosphor Graph app tile and its macOS iconset."""

from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET


assets = Path(__file__).resolve().parent
graph = ET.parse(assets / "phosphor-graph.svg").getroot()
path = graph.find("{http://www.w3.org/2000/svg}path")
assert path is not None and path.get("d"), "Phosphor Graph path is missing"

# Craft apps use Apple's 824/1024 tile grid, with a 112/512 corner radius.
svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <title>Touchcraft</title>
  <rect x="100" y="100" width="824" height="824" rx="180.25" fill="#6554d9"/>
  <g transform="translate(192 202) scale(2.5)" fill="#efe9dc">
    <path d="{path.get('d')}"/>
  </g>
</svg>
'''
master = assets / "touchcraft-icon.svg"
master.write_text(svg)
png = assets / "touchcraft-icon.png"
subprocess.run([
    "magick", "-background", "none", "-density", "288", f"MSVG:{master}",
    "-resize", "1024x1024", f"PNG32:{png}",
], check=True)

with tempfile.TemporaryDirectory(prefix="touchcraft-icon-") as temporary:
    iconset = Path(temporary) / "touchcraft.iconset"
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            suffix = "@2x" if scale == 2 else ""
            subprocess.run([
                "magick", str(png), "-resize", f"{size * scale}x{size * scale}",
                f"PNG32:{iconset / f'icon_{size}x{size}{suffix}.png'}",
            ], check=True)
    subprocess.run([
        "iconutil", "-c", "icns", "-o", str(assets / "touchcraft.icns"), str(iconset),
    ], check=True)

dimensions = subprocess.check_output([
    "magick", "identify", "-format", "%wx%h", str(png),
], text=True)
assert dimensions == "1024x1024", dimensions
alpha = subprocess.check_output([
    "magick", str(png), "-format", "%[fx:p{0,0}.a] %[fx:p{512,150}.a]", "info:",
], text=True)
assert alpha == "0 1", f"Incorrect transparent margin or opaque tile: {alpha}"
assert (assets / "touchcraft.icns").read_bytes().startswith(b"icns")
print("Generated 1024×1024 PNG, vector SVG, and macOS ICNS.")
