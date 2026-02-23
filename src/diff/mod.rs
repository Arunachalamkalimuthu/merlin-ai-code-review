//! Unified diff parser — converts raw patch text into structured [`FileDiff`] trees.
//!
//! Entry point: [`parse_diff`]. The resulting [`FileDiff`] structs are consumed
//! by [`crate::review::ReviewEngine`] and [`crate::digest`].

pub mod parser;

pub use parser::{parse_diff, FileDiff, Hunk, HunkLine, LineKind};
