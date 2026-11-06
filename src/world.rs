//! The starting Self world, written in Self (see world.self).

pub const WORLD_SOURCE: &str = include_str!("world.self");

/// The self-test suite (`selfTest run`), also written in Self.
pub const SELFTEST_SOURCE: &str = include_str!("selftest.self");

/// Morphic: interfaces built from Self objects (morphs), also written in Self.
pub const MORPHIC_SOURCE: &str = include_str!("morphic.self");
