//! Lockfile and manifest parsers.
//!
//! Each submodule turns one registry's file formats into a flat list of
//! [`Dependency`](crate::model::Dependency) values:
//!
//! - [`cargo`] — `Cargo.lock`
//! - [`npm`] — `package.json`
//! - [`pypi`] — `Pipfile.lock`

use std::fmt;

pub mod cargo;
pub mod npm;
pub mod pypi;

/// Why a lockfile or manifest couldn't be parsed.
#[derive(Debug)]
pub enum ParseError {
    /// The file isn't valid JSON, or doesn't have the expected shape.
    Json(serde_json::Error),
    /// The file isn't valid TOML, or doesn't have the expected shape.
    Toml(toml::de::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Json(err) => write!(f, "invalid JSON: {err}"),
            ParseError::Toml(err) => write!(f, "invalid TOML: {err}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Json(err) => Some(err),
            ParseError::Toml(err) => Some(err),
        }
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(err: serde_json::Error) -> Self {
        ParseError::Json(err)
    }
}

impl From<toml::de::Error> for ParseError {
    fn from(err: toml::de::Error) -> Self {
        ParseError::Toml(err)
    }
}
