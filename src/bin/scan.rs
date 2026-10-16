//! Manual CLI for the scanner.
//!
//! Usage:
//!
//! - `scan <lockfile>` — scans a `package-lock.json` or `requirements.txt`
//!   for typosquats and known vulnerabilities (via OSV) and prints the
//!   report. Exits 1 if there are findings, 2 if the scan couldn't run.
//! - `scan <package-name>` — scores a single name against the bundled
//!   popular-package dataset with both typosquat signals.

use std::env;
use std::path::Path;
use std::process::ExitCode;

use deprader::pipeline::{scan_lockfile, ScanOptions};
use deprader::typosquat::scanner::score_package;

fn main() -> ExitCode {
    let target = match env::args().nth(1) {
        Some(target) => target,
        None => {
            eprintln!("usage: scan <lockfile | package-name>");
            return ExitCode::from(2);
        }
    };

    if !Path::new(&target).is_file() {
        println!("{}", score_package(&target));
        return ExitCode::SUCCESS;
    }

    match scan_lockfile(&target, &ScanOptions::default()) {
        Ok(report) => {
            println!("{report}");
            if report.has_findings() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(2)
        }
    }
}
