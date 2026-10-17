//! Parser for npm's `package-lock.json` (lockfile v1, v2 and v3).

use serde::Deserialize;
use std::collections::BTreeMap;

use crate::cve::osv_client::Ecosystem;
use crate::model::Dependency;

/// Errors from parsing a `package-lock.json`.
#[derive(Debug)]
pub enum ParseError {
    /// The input wasn't valid JSON of the expected shape.
    Json(serde_json::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Json(err) => write!(f, "invalid package-lock.json: {err}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Json(err) => Some(err),
        }
    }
}

#[derive(Deserialize)]
struct Lockfile {
    #[serde(rename = "lockfileVersion", default)]
    lockfile_version: u32,
    #[serde(default)]
    packages: BTreeMap<String, Entry>,
    #[serde(default)]
    dependencies: BTreeMap<String, V1Entry>,
}

#[derive(Deserialize)]
struct Entry {
    version: Option<String>,
    #[serde(default)]
    dev: bool,
    #[serde(default)]
    link: bool,
}

/// A lockfile v1 node: an entry plus its recursively nested dependencies.
#[derive(Deserialize)]
struct V1Entry {
    #[serde(flatten)]
    entry: Entry,
    #[serde(default)]
    dependencies: BTreeMap<String, V1Entry>,
}

/// Parses a `package-lock.json` into its resolved dependencies.
///
/// Lockfile v2/v3 are read from the flat `packages` map; v1 (and v2 files
/// without `packages`) from the nested `dependencies` tree. Packages
/// flagged `dev: true` are returned with `direct: false`.
pub fn parse_package_lock(input: &str) -> Result<Vec<Dependency>, ParseError> {
    let lock: Lockfile = serde_json::from_str(input).map_err(ParseError::Json)?;
    let mut deps = Vec::new();

    if lock.lockfile_version >= 2 && !lock.packages.is_empty() {
        for (path, entry) in &lock.packages {
            // "" is the root project; links point at workspace folders.
            if path.is_empty() || entry.link {
                continue;
            }
            // Nested installs look like "node_modules/a/node_modules/@s/b".
            let name = path.rsplit("node_modules/").next().unwrap_or(path);
            push(&mut deps, name, entry);
        }
    } else {
        walk_v1(&lock.dependencies, &mut deps);
    }
    Ok(deps)
}

fn walk_v1(tree: &BTreeMap<String, V1Entry>, deps: &mut Vec<Dependency>) {
    for (name, node) in tree {
        push(deps, name, &node.entry);
        walk_v1(&node.dependencies, deps);
    }
}

fn push(deps: &mut Vec<Dependency>, name: &str, entry: &Entry) {
    if let Some(version) = &entry.version {
        deps.push(Dependency {
            name: name.to_string(),
            version: version.clone(),
            ecosystem: Ecosystem::Npm,
            direct: !entry.dev,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(deps: &'a [Dependency], name: &str, version: &str) -> &'a Dependency {
        deps.iter()
            .find(|d| d.name == name && d.version == version)
            .unwrap_or_else(|| panic!("{name}@{version} not found"))
    }

    #[test]
    fn parses_v3_lockfile() {
        let deps =
            parse_package_lock(include_str!("../../tests/fixtures/package-lock-v3.json")).unwrap();
        assert_eq!(deps.len(), 6);
        assert!(find(&deps, "lodash", "4.17.21").direct);
        // scoped
        assert!(find(&deps, "@babel/core", "7.24.0").direct);
        // nested copy under another package, next to the hoisted one
        assert!(find(&deps, "ms", "2.0.0").direct);
        assert!(find(&deps, "ms", "2.1.3").direct);
        // dev
        assert!(!find(&deps, "jest", "29.7.0").direct);
        // root and link entries skipped
        assert!(deps
            .iter()
            .all(|d| d.name != "my-app" && d.name != "local-pkg"));
    }

    #[test]
    fn parses_v1_lockfile() {
        let deps =
            parse_package_lock(include_str!("../../tests/fixtures/package-lock-v1.json")).unwrap();
        assert_eq!(deps.len(), 6);
        assert!(find(&deps, "lodash", "4.17.21").direct);
        assert!(find(&deps, "@babel/core", "7.24.0").direct);
        assert!(find(&deps, "ms", "2.0.0").direct);
        assert!(find(&deps, "ms", "2.1.3").direct);
        assert!(!find(&deps, "jest", "29.7.0").direct);
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_package_lock("not json").is_err());
    }
}
