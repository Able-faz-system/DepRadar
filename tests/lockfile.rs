//! Fixture tests for the lockfile parsers.

use std::path::PathBuf;

use deprader::lockfile::parse_lockfile;
use deprader::model::Dependency;

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(path)
}

/// `(name, version, direct)` for each parsed dependency, in parse order.
fn relations(dependencies: &[Dependency]) -> Vec<(&str, Option<&str>, bool)> {
    dependencies
        .iter()
        .map(|d| (d.name.as_str(), d.version.as_deref(), d.direct))
        .collect()
}

#[test]
fn npm_marks_root_dependencies_direct_and_nested_installs_transitive() {
    let parsed = parse_lockfile(&fixture("npm-basic/package-lock.json")).unwrap();

    assert_eq!(
        relations(&parsed.dependencies),
        vec![
            ("express", Some("4.21.2"), true),
            // Same name as a direct dependency, but this nested copy is the
            // one express pulled in.
            ("lodash", Some("4.17.4"), false),
            ("expresss", Some("1.0.0"), true),
            ("lodash", Some("4.17.4"), true),
            ("zzz-unversioned-internal-fixture", None, false),
        ]
    );
}

#[test]
fn requirements_txt_dependencies_are_all_direct() {
    let parsed = parse_lockfile(&fixture("pypi-basic/requirements.txt")).unwrap();

    assert_eq!(
        relations(&parsed.dependencies),
        vec![
            ("flask", None, true),
            ("requests", Some("2.19.0"), true),
            ("reqeusts", Some("1.0.0"), true),
        ]
    );
    assert_eq!(parsed.warnings.len(), 1);
}
