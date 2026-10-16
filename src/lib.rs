//! DepRadar off-chain toolkit.
//!
//! This crate hosts the crawler and scanner modules that feed findings to the
//! on-chain bounty router (`contracts/deprader_router`). Only the pieces that
//! have landed are wired up here.
//!
//! [`pipeline::scan_lockfile`] is the end-to-end entry point: it parses a
//! project lockfile ([`lockfile`]) and runs the typosquat ([`typosquat`]) and
//! known-vulnerability ([`cve`]) checks over every dependency in it.

pub mod cve;
pub mod lockfile;
pub mod model;
pub mod pipeline;
pub mod typosquat;

pub use pipeline::{scan_lockfile, Report, ScanOptions};
