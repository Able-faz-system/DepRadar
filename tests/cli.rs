//! End-to-end tests for the `depradar` binary (and the deprecated `scan`
//! alias). Every scan runs with `--offline` so no test touches the network.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn depradar() -> Command {
    Command::cargo_bin("depradar").unwrap()
}

fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

#[test]
fn name_output_matches_the_legacy_scan_binary() {
    let new = depradar().args(["name", "expresss"]).output().unwrap();
    let old = Command::cargo_bin("scan")
        .unwrap()
        .arg("expresss")
        .output()
        .unwrap();

    assert!(new.status.success());
    assert!(old.status.success());
    assert_eq!(
        String::from_utf8_lossy(&new.stdout),
        String::from_utf8_lossy(&old.stdout)
    );
    assert!(String::from_utf8_lossy(&new.stdout).contains("likely typosquat"));
    assert!(String::from_utf8_lossy(&old.stderr).contains("deprecated"));
}

#[test]
fn help_text_matches_snapshot() {
    let output = depradar().arg("--help").output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        include_str!("snapshots/depradar-help.txt"),
        "help text changed; if that's intended, regenerate the snapshot with \
         `cargo run --bin depradar -- --help > tests/snapshots/depradar-help.txt`"
    );
}

#[test]
fn scan_file_text_reports_findings() {
    let output = depradar()
        .args(["scan", "--offline"])
        .arg(fixture("pypi-basic/requirements.txt"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("reqeusts"), "{stdout}");
}

#[test]
fn missing_subcommand_is_a_usage_error() {
    let output = depradar().output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}
