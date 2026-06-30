//! Native host manifest installation support.
//!
//! This module contains the config-driven installer used to write, verify, and
//! remove browser native messaging host manifests. It is available when the
//! crate's `install` feature is enabled, which is included in the default
//! feature set.
//!
//! Most users should call the root re-exports:
//! - [`crate::install()`]
//! - [`crate::verify_installed`]
//! - [`crate::remove`]
//! - [`crate::Scope`]

pub mod manifest;
pub mod paths;

#[cfg(all(windows, feature = "windows-registry"))]
pub mod winreg;

pub use manifest::*;
pub use paths::*;

#[cfg(all(windows, feature = "windows-registry"))]
pub use winreg::*;
