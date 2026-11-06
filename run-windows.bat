@echo off
rem Double-click to build (first time only) and open Self World on Windows.
cd /d "%~dp0"
where cargo >nul 2>nul
if errorlevel 1 (
  echo Self World needs Rust. Install it from https://rustup.rs and run this again.
  pause
  exit /b 1
)
cargo run --release
if errorlevel 1 pause
