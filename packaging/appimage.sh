#!/bin/sh
# Builds Pailune-<version>-x86_64.AppImage in target/appimage/.
# Usage: packaging/appimage.sh   (run from the project folder)
# Needs: cargo, rsvg-convert (librsvg) and appimagetool on PATH, or set
# APPIMAGETOOL=/path/to/appimagetool-x86_64.AppImage
# (https://github.com/AppImage/appimagetool/releases).
# The AppImage runs on systems whose glibc is at least as new as the build
# machine's; build on an older distribution for wider reach.
set -eu
tool="${APPIMAGETOOL:-appimagetool}"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
cargo build --release --locked
out=target/appimage
app="$out/Pailune.AppDir"
rm -rf "$app"
mkdir -p "$app/usr/bin" "$app/usr/share/applications" \
    "$app/usr/share/icons/hicolor/scalable/apps" "$app/usr/share/icons/hicolor/256x256/apps" \
    "$app/usr/share/metainfo"
install -m 755 target/release/pailune "$app/usr/bin/pailune"
install -m 644 packaging/pailune.desktop "$app/usr/share/applications/pailune.desktop"
install -m 644 packaging/pailune.svg "$app/usr/share/icons/hicolor/scalable/apps/pailune.svg"
if command -v rsvg-convert >/dev/null; then
    rsvg-convert -w 256 -h 256 packaging/pailune.svg -o "$app/usr/share/icons/hicolor/256x256/apps/pailune.png"
    cp "$app/usr/share/icons/hicolor/256x256/apps/pailune.png" "$app/pailune.png"
    ln -sf pailune.png "$app/.DirIcon"
fi
install -m 644 packaging/pailune.appdata.xml "$app/usr/share/metainfo/pailune.appdata.xml"
cp "$app/usr/share/applications/pailune.desktop" "$app/pailune.desktop"
ln -sf usr/bin/pailune "$app/AppRun"

if command -v "$tool" >/dev/null; then
    ARCH=x86_64 "$tool" "$app" "$out/Pailune-$version-x86_64.AppImage"
    echo "Built $out/Pailune-$version-x86_64.AppImage"
else
    echo "Staged AppDir at $app."
    echo "Notice: appimagetool not found on PATH. To build the final standalone .AppImage binary, run:"
    echo "  APPIMAGETOOL=/path/to/appimagetool-x86_64.AppImage packaging/appimage.sh"
fi
