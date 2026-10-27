#!/bin/sh
# Installs Pailune for the current user: binary, launcher and icon.
# Usage: packaging/install.sh  (run from the project folder after `cargo build --release`)
set -eu
bin="${HOME}/.local/bin"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"
mkdir -p "$bin" "$apps" "$icons"
install -m 755 target/release/pailune "$bin/pailune"
install -m 644 packaging/pailune.desktop "$apps/pailune.desktop"
install -m 644 packaging/pailune.svg "$icons/pailune.svg"
command -v update-desktop-database >/dev/null && update-desktop-database "$apps" || true
echo "Installed to $bin/pailune. Make sure $bin is on your PATH."
echo "If you use Pailune as nxm:// handler, register it again from the Nexus page so it points here."
