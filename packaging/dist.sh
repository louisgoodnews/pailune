#!/bin/sh
# Creates standard Linux distribution archives for Pailune:
# - target/dist/pailune-v<version>-x86_64-unknown-linux-gnu.tar.gz
# - target/dist/SHA256SUMS
# - target/appimage/Pailune.AppDir (and AppImage if appimagetool is present)
#
# Usage: packaging/dist.sh
set -eu

cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
arch="x86_64-unknown-linux-gnu"
pkg_name="pailune-v${version}-${arch}"
dist_dir="target/dist"
stage_dir="${dist_dir}/${pkg_name}"

echo "Building release binary for Pailune v${version}..."
cargo build --release --locked

rm -rf "${stage_dir}"
mkdir -p "${stage_dir}/bin" "${stage_dir}/packaging" "${dist_dir}"

echo "Staging release files..."
install -m 755 target/release/pailune "${stage_dir}/bin/pailune"
if command -v strip >/dev/null; then
    strip "${stage_dir}/bin/pailune"
fi

[ -f LICENSE ] && cp LICENSE "${stage_dir}/"
[ -f LICENSE-MIT ] && cp LICENSE-MIT "${stage_dir}/"
[ -f LICENSE-APACHE ] && cp LICENSE-APACHE "${stage_dir}/"
cp README.md "${stage_dir}/"
cp packaging/install.sh "${stage_dir}/"
chmod +x "${stage_dir}/install.sh"
cp packaging/pailune.desktop "${stage_dir}/packaging/"
cp packaging/pailune.svg "${stage_dir}/packaging/"
cp packaging/pailune.appdata.xml "${stage_dir}/packaging/"

echo "Creating tarball: ${dist_dir}/${pkg_name}.tar.gz..."
tar -C "${dist_dir}" -czf "${dist_dir}/${pkg_name}.tar.gz" "${pkg_name}"

echo "Staging AppImage..."
packaging/appimage.sh

echo "Generating SHA256 checksums..."
(
    cd "${dist_dir}"
    sha256sum "${pkg_name}.tar.gz" > SHA256SUMS
    if [ -f "../appimage/Pailune-${version}-x86_64.AppImage" ]; then
        cp "../appimage/Pailune-${version}-x86_64.AppImage" .
        sha256sum "Pailune-${version}-x86_64.AppImage" >> SHA256SUMS
    fi
)

echo "=== Packaging Complete ==="
ls -lh "${dist_dir}/${pkg_name}.tar.gz"
cat "${dist_dir}/SHA256SUMS"
