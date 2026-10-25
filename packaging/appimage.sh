#!/bin/sh
# Builds Pylune-<version>-x86_64.AppImage in target/appimage/.
# Usage: packaging/appimage.sh   (run from the project folder)
# Needs: cargo, rsvg-convert (librsvg) and appimagetool on PATH, or set
# APPIMAGETOOL=/path/to/appimagetool-x86_64.AppImage
# (https://github.com/AppImage/appimagetool/releases).
# The AppImage runs on systems whose glibc is at least as new as the build
# machine's; build on an older distribution for wider reach.
set -eu
tool="${APPIMAGETOOL:-appimagetool}"
command -v "$tool" >/dev/null || { echo "appimagetool not found (set APPIMAGETOOL)" >&2; exit 1; }
command -v rsvg-convert >/dev/null || { echo "rsvg-convert not found (install librsvg)" >&2; exit 1; }
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
cargo build --release --locked
out=target/appimage
app="$out/Pylune.AppDir"
rm -rf "$app"
mkdir -p "$app/usr/bin" "$app/usr/share/applications" \
    "$app/usr/share/icons/hicolor/scalable/apps" "$app/usr/share/icons/hicolor/256x256/apps" \
    "$app/usr/share/metainfo"
install -m 755 target/release/pylune "$app/usr/bin/pylune"
install -m 644 packaging/pylune.desktop "$app/usr/share/applications/pylune.desktop"
install -m 644 packaging/pylune.svg "$app/usr/share/icons/hicolor/scalable/apps/pylune.svg"
rsvg-convert -w 256 -h 256 packaging/pylune.svg -o "$app/usr/share/icons/hicolor/256x256/apps/pylune.png"
install -m 644 packaging/pylune.appdata.xml "$app/usr/share/metainfo/pylune.appdata.xml"
cp "$app/usr/share/applications/pylune.desktop" "$app/pylune.desktop"
cp "$app/usr/share/icons/hicolor/256x256/apps/pylune.png" "$app/pylune.png"
ln -s pylune.png "$app/.DirIcon"
ln -s usr/bin/pylune "$app/AppRun"
ARCH=x86_64 "$tool" "$app" "$out/Pylune-$version-x86_64.AppImage"
echo "Built $out/Pylune-$version-x86_64.AppImage"
