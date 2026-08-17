//! Shared building blocks for the `*me` command-line tools.
//!
//! Keep action-specific argument handling in `src/bin/<action>.rs` and move
//! reusable operating-system integrations into modules like this one.

pub mod clipboard;
pub mod installer;
