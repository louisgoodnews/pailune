#!/bin/sh
# Installs Pailune for the current user: binary, launcher and icon.
# Usage:
#   From repository: packaging/install.sh (after `cargo build --release`)
#   From release tarball: ./install.sh
set -eu

cd "$(dirname "$0")"

bin_src=""
desktop_src=""
svg_src=""

if [ -f "bin/pailune" ]; then
    bin_src="bin/pailune"
    desktop_src="packaging/pailune.desktop"
    svg_src="packaging/pailune.svg"
elif [ -f "../target/release/pailune" ]; then
    bin_src="../target/release/pailune"
    desktop_src="pailune.desktop"
    svg_src="pailune.svg"
elif [ -f "target/release/pailune" ]; then
    bin_src="target/release/pailune"
    desktop_src="packaging/pailune.desktop"
    svg_src="packaging/pailune.svg"
else
    echo "Error: pailune binary not found. Build with 'cargo build --release' first." >&2
    exit 1
fi

bin="${HOME}/.local/bin"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
mkdir -p "$bin" "$apps" "$icons"

install -m 755 "$bin_src" "$bin/pailune"
install -m 644 "$desktop_src" "$apps/pailune.desktop"
install -m 644 "$svg_src" "$icons/pailune.svg"
command -v update-desktop-database >/dev/null && update-desktop-database "$apps" || true

echo "Installed to $bin/pailune. Make sure $bin is on your PATH."
echo "If you use Pailune as nxm:// handler, register it again from the Nexus page so it points here."
