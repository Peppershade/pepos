#!/bin/bash
# pepos ISO builder
# Wraps void-mklive to produce a bootable pepos ISO.
# Run this on a Void Linux machine with void-mklive installed.
#
# Usage: ./iso/build.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT="$REPO_ROOT/pepos.iso"

echo "==> Building pepos ISO"

# 1. Compile all pepos components
echo "==> Compiling pepos components"
cd "$REPO_ROOT"
cargo build --release

# 2. Stage binaries into the overlay
echo "==> Staging binaries"
mkdir -p "$SCRIPT_DIR/overlay/usr/local/bin"
cp target/release/pepos-compositor  "$SCRIPT_DIR/overlay/usr/local/bin/"
cp target/release/pepos-menubar      "$SCRIPT_DIR/overlay/usr/local/bin/"
cp target/release/pepos-dock         "$SCRIPT_DIR/overlay/usr/local/bin/"
cp target/release/pepos-launcher     "$SCRIPT_DIR/overlay/usr/local/bin/"

# 3. Build the ISO with void-mklive
echo "==> Running void-mklive"
sudo void-mklive \
    -a x86_64 \
    -o "$OUTPUT" \
    -p "sway elogind dbus polkit xorg-server-xwayland" \
    -I "$SCRIPT_DIR/overlay"

echo "==> Done: $OUTPUT"
