//! Python (PyPI) lockfile and manifest parsers: `Pipfile.lock`,
//! `requirements.txt` and `poetry.lock`.

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

/// Parses a `requirements.txt`, discarding any warnings. See
/// [`parse_requirements_with_warnings`].
pub fn parse_requirements(contents: &str) -> Result<Vec<Dependency>, ParseError> {
    parse_requirements_with_warnings(contents).map(|(deps, _)| deps)
}

/// Parses a pip `requirements.txt` into its declared dependencies, plus any
/// warnings about lines that were skipped.
///
/// - `name==version` (and `===`) pins give `version: Some(..)`; any other
///   specifier (`>=`, `~=`, `<`, `!=`, `==1.*`) or a bare name gives
///   `version: None`.
/// - Comments, blank lines, `--hash` options, extras (`name[extra]`) and
///   environment markers (`; python_version < "3.9"`) are ignored.
/// - `-r other.txt` / `--requirement other.txt` includes are not followed;
///   each one is reported as a warning. Other option lines (`-e`, `-c`,
///   `--index-url`, ...) are skipped silently.
///
/// Every entry is `direct: true`, since the file lists what the project
/// asks for.
pub fn parse_requirements_with_warnings(
    contents: &str,
) -> Result<(Vec<Dependency>, Vec<String>), ParseError> {
    let mut deps = Vec::new();
    let mut warnings = Vec::new();

    for line in logical_lines(contents) {
        if line.is_empty() {
            continue;
        }
        if let Some(include) = include_target(&line) {
            warnings.push(format!(
                "requirements include \"{include}\" was not followed"
            ));
            continue;
        }
        if line.starts_with('-') {
            continue;
        }
        if let Some((name, version)) = parse_requirement(&line) {
            deps.push(Dependency {
                name,
                version,
                ecosystem: Ecosystem::PyPI,
                direct: true,
                dev: false,
            });
        }
    }
    Ok((deps, warnings))
}

/// Joins `\`-continued lines and strips comments, returning one trimmed
/// logical requirement per entry.
fn logical_lines(contents: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut pending = String::new();
    for raw in contents.lines() {
        let mut line = raw;
        // A `#` starts a comment at the start of a line or after whitespace.
        if let Some(i) = line
            .char_indices()
            .find(|&(i, c)| c == '#' && (i == 0 || line[..i].ends_with(char::is_whitespace)))
            .map(|(i, _)| i)
        {
            line = &line[..i];
        }
        let line = line.trim_end();
        if let Some(stripped) = line.strip_suffix('\\') {
            pending.push_str(stripped);
            pending.push(' ');
        } else {
            pending.push_str(line);
            lines.push(pending.trim().to_string());
            pending.clear();
        }
    }
    if !pending.trim().is_empty() {
        lines.push(pending.trim().to_string());
    }
    lines
}

/// Returns the file named by a `-r` / `--requirement` line.
fn include_target(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("--requirement")
        .or_else(|| line.strip_prefix("-r"))?;
    let target = rest.trim_start_matches(['=', ' ', '\t']).trim();
    (!target.is_empty()).then_some(target)
}

/// Splits one requirement line into `(name, pinned version)`.
fn parse_requirement(line: &str) -> Option<(String, Option<String>)> {
    // Drop environment markers, then any trailing `--hash=...` options.
    let spec = line.split(';').next()?;
    let spec = spec.split(" --").next()?.trim();

    let name_end = spec
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .unwrap_or(spec.len());
    let name = &spec[..name_end];
    if name.is_empty() {
        return None;
    }

    // Skip extras (`name[security]`) and whitespace before the specifier.
    let mut rest = spec[name_end..].trim_start();
    if rest.starts_with('[') {
        rest = rest[rest.find(']')? + 1..].trim_start();
    }

    let version = rest
        .strip_prefix("===")
        .or_else(|| rest.strip_prefix("=="))
        .map(str::trim)
        .filter(|v| !v.is_empty() && !v.contains(['*', ',', ' ']))
        .map(str::to_string);
    Some((name.to_string(), version))
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

    #[test]
    fn requirements_pins_and_unpinned_specifiers() {
        let (deps, warnings) = parse_requirements_with_warnings(
            "requests==2.31.0\nflask>=2.0\nnumpy~=1.26\ndjango\npkg===1.0.0\nwild==1.*\n",
        )
        .unwrap();
        let got: Vec<_> = deps
            .iter()
            .map(|d| (d.name.as_str(), d.version.as_deref()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("requests", Some("2.31.0")),
                ("flask", None),
                ("numpy", None),
                ("django", None),
                ("pkg", Some("1.0.0")),
                ("wild", None),
            ]
        );
        assert!(warnings.is_empty());
        assert!(deps
            .iter()
            .all(|d| d.direct && !d.dev && d.ecosystem == Ecosystem::PyPI));
    }

    #[test]
    fn requirements_ignore_comments_and_blank_lines() {
        let deps = parse_requirements(
            "# top comment\n\n   \nrequests==2.0  # inline comment\n  # indented comment\n",
        )
        .unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "requests");
        assert_eq!(deps[0].version.as_deref(), Some("2.0"));
    }

    #[test]
    fn requirements_ignore_hash_options() {
        let deps = parse_requirements(
            "requests==2.31.0 \\\n    --hash=sha256:aaa \\\n    --hash=sha256:bbb\nflask==3.0.0 --hash=sha256:ccc\n",
        )
        .unwrap();
        let got: Vec<_> = deps
            .iter()
            .map(|d| (d.name.as_str(), d.version.as_deref()))
            .collect();
        assert_eq!(got, vec![("requests", Some("2.31.0")), ("flask", Some("3.0.0"))]);
    }

    #[test]
    fn requirements_ignore_environment_markers_and_extras() {
        let deps = parse_requirements(
            "tomli==2.0.1 ; python_version < \"3.11\"\nrequests[security]==2.31.0\nbackport>=1.0; sys_platform == 'win32'\n",
        )
        .unwrap();
        let got: Vec<_> = deps
            .iter()
            .map(|d| (d.name.as_str(), d.version.as_deref()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("tomli", Some("2.0.1")),
                ("requests", Some("2.31.0")),
                ("backport", None),
            ]
        );
    }

    #[test]
    fn requirements_report_includes_as_warnings_without_following() {
        let (deps, warnings) = parse_requirements_with_warnings(
            "-r base.txt\n--requirement=dev.txt\n-e .\n--index-url https://example.com\nrequests==2.0\n",
        )
        .unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("base.txt"));
        assert!(warnings[1].contains("dev.txt"));
    }
}
