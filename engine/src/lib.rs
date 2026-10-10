//! Raftaar engine: every change Raftaar makes to Windows lives here.
//!
//! The crate has no UI dependency, so the same engine can later run as the
//! per-PC agent in cafe mode. All system access goes through the `Registry`
//! and `Shell` traits, which keeps the logic testable with mocks.

pub mod catalog;
pub mod error;
pub mod power;
pub mod restore;
pub mod snapshot;
pub mod sys;
pub mod tweak;

pub use error::{Error, Result};
