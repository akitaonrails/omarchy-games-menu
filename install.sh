#!/bin/sh
# Build ogm and install the launcher entry points into ~/.local/bin.
set -eu

REPO="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$HOME/.local/bin"

cargo build --release --manifest-path "$REPO/Cargo.toml"

ln -sf "$REPO/target/release/ogm" "$BIN_DIR/ogm"
ln -sf "$REPO/games-menu" "$BIN_DIR/games-menu"

echo "Installed:"
echo "  $BIN_DIR/ogm        -> $REPO/target/release/ogm"
echo "  $BIN_DIR/games-menu -> $REPO/games-menu"
echo
echo "First run:"
echo "  ogm scan       # build the game collection"
echo "  ogm refresh    # fetch GitHub releases + SteamGridDB covers"
echo "  games-menu     # open the grid overlay"
