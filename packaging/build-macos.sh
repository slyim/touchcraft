#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --offline --release
bundle="$PWD/dist/Touchcraft.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp target/release/touchcraft "$bundle/Contents/MacOS/Touchcraft"
cp assets/touchcraft.icns "$bundle/Contents/Resources/touchcraft.icns"
cp packaging/Info.plist "$bundle/Contents/Info.plist"
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$bundle"
fi
printf 'Built %s\n' "$bundle"
