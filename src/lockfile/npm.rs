//! Parser for npm `package-lock.json` (lockfile v2 and v3).
//!
//! v2/v3 lockfiles carry a flat `packages` map keyed by install location:
//! `""` is the project itself, `node_modules/foo` is a top-level install and
//! `node_modules/foo/node_modules/bar` is a copy of `bar` nested under `foo`.
//! Every entry other than the root is one installed package. v1 lockfiles
//! (npm 6 and older) have no `packages` map and aren't supported.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{LockfileError, ParsedLockfile};
use crate::model::{Dependency, Ecosystem};

const NODE_MODULES: &str = "node_modules/";

#[derive(Debug, Deserialize)]
struct PackageLock {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<u32>,
    #[serde(default)]
    packages: BTreeMap<String, PackageEntry>,
}

#[derive(Debug, Deserialize)]
struct PackageEntry {
    /// Set when the install location's name differs from the real package
    /// name (npm aliases, `"foo": "npm:bar@1.0.0"`).
    name: Option<String>,
    version: Option<String>,
    /// A symlink to a workspace folder; the target is listed separately.
    #[serde(default)]
    link: bool,
}

/// Parses the contents of a `package-lock.json`.
pub fn parse(contents: &str) -> Result<ParsedLockfile, LockfileError> {
    let lock: PackageLock = serde_json::from_str(contents)
        .map_err(|err| LockfileError::Invalid(format!("package-lock.json: {err}")))?;

    let mut dependencies = Vec::new();
    let mut warnings = Vec::new();

    if lock.packages.is_empty() {
        let version = lock
            .lockfile_version
            .map_or_else(|| "unknown".to_string(), |v| v.to_string());
        warnings.push(format!(
            "package-lock.json lockfileVersion {version} has no `packages` map; \
             only lockfile v2/v3 are supported, so no dependencies were scanned"
        ));
    }

    for (key, entry) in &lock.packages {
        // The root entry is the project itself, and entries outside
        // node_modules/ are local workspace sources: neither is a third-party
        // install.
        if key.is_empty() || entry.link {
            continue;
        }
        let Some(name) = package_name(key, entry) else {
            continue;
        };

        let version = match &entry.version {
            Some(version) if is_registry_version(version) => Some(version.clone()),
            Some(version) => {
                warnings.push(format!(
                    "{name} (`{key}`) resolves to `{version}`, not a registry version; \
                     skipping vulnerability check"
                ));
                None
            }
            None => {
                warnings.push(format!(
                    "{name} (`{key}`) has no version; skipping vulnerability check"
                ));
                None
            }
        };

        dependencies.push(Dependency {
            name,
            version,
            ecosystem: Ecosystem::Npm,
        });
    }

    Ok(ParsedLockfile {
        ecosystem: Ecosystem::Npm,
        dependencies,
        warnings,
    })
}

/// The package name for a `packages` entry: the explicit `name` if the lock
/// records one, otherwise the path segment(s) after the last `node_modules/`
/// (which keeps scoped names like `@babel/core` whole). `None` for entries
/// that aren't under `node_modules/`.
fn package_name(key: &str, entry: &PackageEntry) -> Option<String> {
    let start = key.rfind(NODE_MODULES)? + NODE_MODULES.len();
    let name = entry.name.as_deref().unwrap_or(&key[start..]);
    (!name.is_empty()).then(|| name.to_string())
}

/// False for versions npm records for non-registry installs: git URLs,
/// tarball URLs and `file:` paths. OSV can't match those.
fn is_registry_version(version: &str) -> bool {
    !version.contains(':') && !version.contains('/')
}
