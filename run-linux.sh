#!/bin/sh
# Build (first time only) and open Self World on Linux.
# Needs Rust (https://rustup.rs) and, on Debian/Ubuntu, the packages
# build-essential libxkbcommon-dev libgl1-mesa-dev libwayland-dev libx11-dev
cd "$(dirname "$0")" || exit 1
command -v cargo >/dev/null 2>&1 || . "$HOME/.cargo/env" 2>/dev/null
if ! command -v cargo >/dev/null 2>&1; then
  echo "Self World needs Rust. Install it from https://rustup.rs and run this again."
  exit 1
fi
exec cargo run --release -- "$@"
