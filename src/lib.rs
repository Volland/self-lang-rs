//! Self World: an implementation of the Self programming language in Rust,
//! with a modern outliner-based desktop that runs natively and on the web.

pub mod app;
pub mod lexer;
pub mod morphic;
pub mod parser;
#[cfg(not(target_arch = "wasm32"))]
pub mod png;
pub mod vm;
pub mod world;

#[cfg(target_arch = "wasm32")]
pub mod web;
