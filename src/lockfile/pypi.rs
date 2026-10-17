//! Python (PyPI) lockfile parsers.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::ParseError;
use crate::model::{Dependency, Ecosystem};

/// The parts of a `Pipfile.lock` we read: the `default` (runtime) and
/// `develop` (dev-only) package maps. `_meta` and per-package hashes,
/// markers and indexes are ignored.
#[derive(Debug, Deserialize)]
struct PipfileLock {
    #[serde(default)]
    default: BTreeMap<String, PipfileLockEntry>,
    #[serde(default)]
    develop: BTreeMap<String, PipfileLockEntry>,
}

#[derive(Debug, Deserialize)]
struct PipfileLockEntry {
    /// Pinned version such as `"==2.31.0"`. Absent for VCS, path and
    /// editable installs.
    version: Option<String>,
}

/// Parses a Pipenv `Pipfile.lock` into its locked dependencies.
///
/// Both the `default` and `develop` sections are read; `develop` entries are
/// marked `dev: true`. The `==` (or `===`) pin operator is stripped from each
/// version. `Pipfile.lock` lists transitive packages alongside the ones
/// declared in the `Pipfile` without telling them apart, so `direct` is
/// always `false`.
pub fn parse_pipfile_lock(contents: &str) -> Result<Vec<Dependency>, ParseError> {
    let lock: PipfileLock = serde_json::from_str(contents)?;

    let default = lock.default.into_iter().map(|entry| (entry, false));
    let develop = lock.develop.into_iter().map(|entry| (entry, true));

    Ok(default
        .chain(develop)
        .map(|((name, entry), dev)| Dependency {
            name,
            version: entry.version.map(|v| strip_pin(&v).to_string()),
            ecosystem: Ecosystem::PyPI,
            direct: false,
            dev,
        })
        .collect())
}

/// Strips a leading `===` or `==` pin operator from a version string.
fn strip_pin(version: &str) -> &str {
    let version = version.trim();
    version
        .strip_prefix("===")
        .or_else(|| version.strip_prefix("=="))
        .unwrap_or(version)
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIPFILE_LOCK: &str = include_str!("../../tests/fixtures/Pipfile.lock");

    fn find<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
        deps.iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("{name} not found in {deps:?}"))
    }

    #[test]
    fn parses_default_and_develop_sections() {
        let deps = parse_pipfile_lock(PIPFILE_LOCK).unwrap();
        assert_eq!(deps.len(), 8);
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::PyPI));
        assert!(deps.iter().all(|d| !d.direct));

        let requests = find(&deps, "requests");
        assert_eq!(requests.version.as_deref(), Some("2.31.0"));
        assert!(!requests.dev);

        let urllib3 = find(&deps, "urllib3");
        assert_eq!(urllib3.version.as_deref(), Some("2.2.1"));
        assert!(!urllib3.dev);

        let pytest = find(&deps, "pytest");
        assert_eq!(pytest.version.as_deref(), Some("8.1.1"));
        assert!(pytest.dev);

        let black = find(&deps, "black");
        assert_eq!(black.version.as_deref(), Some("24.3.0"));
        assert!(black.dev);
    }

    #[test]
    fn entries_without_a_version_keep_none() {
        let deps = parse_pipfile_lock(PIPFILE_LOCK).unwrap();
        let vcs = find(&deps, "my-git-package");
        assert_eq!(vcs.version, None);
        assert!(!vcs.dev);
    }

    #[test]
    fn missing_sections_are_empty() {
        assert!(parse_pipfile_lock(r#"{"_meta": {}}"#).unwrap().is_empty());
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(parse_pipfile_lock("not json").is_err());
    }

    #[test]
    fn strip_pin_handles_both_operators() {
        assert_eq!(strip_pin("==1.0.0"), "1.0.0");
        assert_eq!(strip_pin("===1.0.0"), "1.0.0");
        assert_eq!(strip_pin("1.0.0"), "1.0.0");
    }
}
