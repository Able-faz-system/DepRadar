//! Parser for pip `requirements.txt` files.
//!
//! Handles the common subset of the format: one requirement per line,
//! `#` comments, `\` line continuations, extras (`requests[socks]`),
//! environment markers (`; python_version < "3.8"`), per-requirement options
//! such as `--hash=...`, and `name @ url` direct references. Global options
//! (`-r other.txt`, `-e .`, `--index-url ...`) aren't followed; each one
//! becomes a warning.
//!
//! Only `==` / `===` pins count as a version. Anything looser (`>=`, `~=`,
//! `==2.*`) is recorded without one, since OSV needs an exact version.
//!
//! A requirements file only lists what the project asked for, so every
//! dependency in it is direct.

use super::ParsedLockfile;
use crate::model::{Dependency, Ecosystem};

/// Parses the contents of a `requirements.txt`. Never fails: lines it can't
/// understand become warnings.
pub fn parse(contents: &str) -> ParsedLockfile {
    let mut dependencies = Vec::new();
    let mut warnings = Vec::new();

    for (line_no, line) in logical_lines(contents) {
        let line = strip_comment(&line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('-') {
            warnings.push(format!("line {line_no}: option `{line}` is not followed"));
            continue;
        }
        match parse_requirement(line) {
            Some(dependency) => dependencies.push(dependency),
            None => warnings.push(format!("line {line_no}: could not parse `{line}`")),
        }
    }

    ParsedLockfile {
        ecosystem: Ecosystem::PyPI,
        dependencies,
        warnings,
    }
}

/// Joins `\`-continued lines, yielding each logical line with the 1-based
/// number of the physical line it starts on.
fn logical_lines(contents: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    let mut current: Option<(usize, String)> = None;

    for (index, raw) in contents.lines().enumerate() {
        let (start, mut text) = current.take().unwrap_or((index + 1, String::new()));
        match raw.strip_suffix('\\') {
            Some(continued) => {
                text.push_str(continued);
                text.push(' ');
                current = Some((start, text));
            }
            None => {
                text.push_str(raw);
                lines.push((start, text));
            }
        }
    }
    if let Some(last) = current {
        lines.push(last);
    }
    lines
}

/// Drops a `#` comment: one at the start of the line, or one preceded by
/// whitespace (a `#` inside a URL fragment is left alone).
fn strip_comment(line: &str) -> &str {
    let mut previous_is_space = true;
    for (index, c) in line.char_indices() {
        if c == '#' && previous_is_space {
            return &line[..index];
        }
        previous_is_space = c.is_whitespace();
    }
    line
}

/// Parses one requirement line into a dependency, or `None` if it doesn't
/// start with a valid project name.
fn parse_requirement(line: &str) -> Option<Dependency> {
    // Per-requirement options (`--hash=...`) and environment markers don't
    // affect which package or version is installed.
    let spec = line.split(" --").next().unwrap_or(line);
    let spec = spec.split(';').next().unwrap_or(spec).trim();

    let name_len = spec
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .unwrap_or(spec.len());
    let name = &spec[..name_len];
    if !name.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return None;
    }

    let mut rest = spec[name_len..].trim_start();
    if rest.starts_with('[') {
        let close = rest.find(']')?;
        rest = rest[close + 1..].trim_start();
    }

    // `name @ https://...` is a direct reference, not a registry version.
    let version = if rest.starts_with('@') {
        None
    } else {
        pinned_version(rest)
    };

    Some(Dependency {
        name: normalize_name(name),
        version,
        ecosystem: Ecosystem::PyPI,
        direct: true,
    })
}

/// The exact version from a `==X` or `===X` specifier, if that's the only
/// specifier and it has no wildcard.
fn pinned_version(specifier: &str) -> Option<String> {
    let version = specifier
        .strip_prefix("===")
        .or_else(|| specifier.strip_prefix("=="))?
        .trim();
    let exact = !version.is_empty() && !version.contains(['*', ',', ' ']);
    exact.then(|| version.to_string())
}

/// PEP 503 name normalization: lowercase, with runs of `-`, `_` and `.`
/// collapsed to a single `-`. `Flask`, `flask` and `FLASK` are one project.
fn normalize_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut in_separator_run = false;
    for c in name.chars() {
        if matches!(c, '-' | '_' | '.') {
            if !in_separator_run {
                normalized.push('-');
            }
            in_separator_run = true;
        } else {
            normalized.push(c.to_ascii_lowercase());
            in_separator_run = false;
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dep(name: &str, version: Option<&str>) -> Dependency {
        Dependency {
            name: name.to_string(),
            version: version.map(str::to_string),
            ecosystem: Ecosystem::PyPI,
            direct: true,
        }
    }

    #[test]
    fn parses_pins_ranges_extras_and_markers() {
        let parsed = parse(
            "# pinned\n\
             requests==2.19.0\n\
             Flask>=2.0  # not pinned\n\
             urllib3[socks]==1.24.1 ; python_version >= \"3.6\"\n\
             Django===4.2.1\n\
             numpy==1.*\n",
        );
        assert_eq!(
            parsed.dependencies,
            vec![
                dep("requests", Some("2.19.0")),
                dep("flask", None),
                dep("urllib3", Some("1.24.1")),
                dep("django", Some("4.2.1")),
                dep("numpy", None),
            ]
        );
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn handles_continuations_hashes_and_direct_references() {
        let parsed = parse(
            "six==1.16.0 \\\n    --hash=sha256:abc\n\
             mypkg @ https://example.com/mypkg-1.0.tar.gz#sha256=def\n",
        );
        assert_eq!(
            parsed.dependencies,
            vec![dep("six", Some("1.16.0")), dep("mypkg", None)]
        );
    }

    #[test]
    fn options_and_garbage_become_warnings() {
        let parsed = parse("-r base.txt\n--index-url https://x\n==1.0\nrequests\n");
        assert_eq!(parsed.dependencies, vec![dep("requests", None)]);
        assert_eq!(parsed.warnings.len(), 3);
        assert!(parsed.warnings[0].starts_with("line 1:"));
        assert!(parsed.warnings[2].starts_with("line 3:"));
    }

    #[test]
    fn normalizes_names_per_pep_503() {
        assert_eq!(normalize_name("Zope.Interface"), "zope-interface");
        assert_eq!(normalize_name("python__dateutil"), "python-dateutil");
    }
}
