//! Lockfile parsers: turn a project's dependency manifest into a flat list
//! of [`Dependency`] entries the scanner can check.
//!
//! Supported formats:
//!
//! - npm `package-lock.json` / `npm-shrinkwrap.json`, lockfile v2 and v3
//!   ([`npm`])
//! - pip `requirements*.txt` ([`pypi`])
//!
//! Parsers are lenient: an entry they can't make sense of becomes a warning
//! in [`ParsedLockfile::warnings`] rather than failing the whole file. Only a
//! file that can't be read or isn't the expected shape at all is an error.

pub mod npm;
pub mod pypi;

use std::fmt;
use std::path::Path;

use crate::model::{Dependency, Ecosystem};

/// The result of parsing one lockfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLockfile {
    pub ecosystem: Ecosystem,
    pub dependencies: Vec<Dependency>,
    /// Entries that were skipped or only partly understood.
    pub warnings: Vec<String>,
}

/// Errors that stop a lockfile from being parsed at all.
#[derive(Debug)]
pub enum LockfileError {
    /// The file couldn't be read.
    Io(std::io::Error),
    /// The file name doesn't match any supported lockfile format.
    UnknownFormat(String),
    /// The file is a supported format but its contents are malformed.
    Invalid(String),
}

impl fmt::Display for LockfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockfileError::Io(err) => write!(f, "failed to read lockfile: {err}"),
            LockfileError::UnknownFormat(name) => write!(
                f,
                "unsupported lockfile `{name}` (expected package-lock.json, \
                 npm-shrinkwrap.json or requirements*.txt)"
            ),
            LockfileError::Invalid(reason) => write!(f, "malformed lockfile: {reason}"),
        }
    }
}

impl std::error::Error for LockfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockfileError::Io(err) => Some(err),
            LockfileError::UnknownFormat(_) | LockfileError::Invalid(_) => None,
        }
    }
}

/// Parses the lockfile at `path`, picking the parser from the file name.
pub fn parse_lockfile(path: &Path) -> Result<ParsedLockfile, LockfileError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if file_name == "package-lock.json" || file_name == "npm-shrinkwrap.json" {
        let contents = std::fs::read_to_string(path).map_err(LockfileError::Io)?;
        npm::parse(&contents)
    } else if file_name.starts_with("requirements") && file_name.ends_with(".txt") {
        let contents = std::fs::read_to_string(path).map_err(LockfileError::Io)?;
        Ok(pypi::parse(&contents))
    } else {
        Err(LockfileError::UnknownFormat(file_name.to_string()))
    }
}
