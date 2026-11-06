#!/bin/bash
# Double-click to build (first time only) and open Self World on macOS.
cd "$(dirname "$0")"
if ! command -v cargo >/dev/null 2>&1; then
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "Self World needs Rust. Install it from https://rustup.rs and run this again."
  read -r -p "Press Enter to close."
  exit 1
fi
cargo run --release
