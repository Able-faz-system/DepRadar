//! `depradar` — check package names and project lockfiles for typosquats and
//! known vulnerabilities.
//!
//! Run `depradar --help` for usage.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use deprader::ignore::{IgnoreFile, IGNORE_FILE_NAME};
use deprader::pipeline::{scan_dir, scan_lockfile, DirReport, Report, ScanOptions};
use deprader::typosquat::scanner::score_package;

const EXAMPLES: &str = "\
Examples:
  depradar name expresss                 Check one package name for typosquat signals
  depradar scan package-lock.json        Scan a lockfile for typosquats and known CVEs
  depradar scan .                        Scan every lockfile in a directory or monorepo
  depradar scan . --offline              Typosquat checks only, no OSV lookup

Exit codes:
  0  No findings
  1  At least one finding (`scan` only; suppressed findings don't count)
  2  Usage error, or the scan couldn't complete";

#[derive(Debug, Parser)]
#[command(
    name = "depradar",
    version,
    about = "Check package names and lockfiles for typosquats and known vulnerabilities",
    after_help = EXAMPLES
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Score a single package name against the popular-package dataset
    Name {
        /// The package name to check, e.g. `expresss`
        package: String,
    },
    /// Scan a lockfile, or every lockfile under a directory
    Scan {
        /// A lockfile (package-lock.json, npm-shrinkwrap.json,
        /// requirements*.txt) or a directory to search for them
        path: PathBuf,
        /// Skip the OSV vulnerability lookup (typosquat checks only)
        #[arg(long)]
        offline: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Name { package } => {
            println!("{}", score_package(&package));
            ExitCode::SUCCESS
        }
        Command::Scan { path, offline } => match scan(&path, offline) {
            Ok(scanned) => {
                match &scanned {
                    Scanned::File(report) => println!("{report}"),
                    Scanned::Dir(report) => println!("{report}"),
                }
                if scanned.has_findings() {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(message) => {
                eprintln!("error: {message}");
                ExitCode::from(2)
            }
        },
    }
}

/// What `depradar scan` scanned: one lockfile, or a directory tree.
enum Scanned {
    File(Report),
    Dir(DirReport),
}

impl Scanned {
    fn has_findings(&self) -> bool {
        match self {
            Scanned::File(report) => report.has_findings(),
            Scanned::Dir(report) => report.has_findings(),
        }
    }
}

/// Scans `path` as a single lockfile or, for a directory, every lockfile
/// under it, honouring a `.depradarignore` next to the lockfile (or at the
/// directory's root).
fn scan(path: &Path, offline: bool) -> Result<Scanned, String> {
    let ignore_dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new(""))
    };
    let ignore_path = ignore_dir.join(IGNORE_FILE_NAME);
    let ignore = if ignore_path.is_file() {
        Some(IgnoreFile::load(&ignore_path).map_err(|err| err.to_string())?)
    } else {
        None
    };

    let mut opts = ScanOptions {
        ignore: ignore.as_ref(),
        ..ScanOptions::default()
    };
    if offline {
        opts.advisories = None;
    }

    if path.is_dir() {
        scan_dir(path, &opts)
            .map(Scanned::Dir)
            .map_err(|err| format!("{}: {err}", path.display()))
    } else {
        scan_lockfile(path, &opts)
            .map(Scanned::File)
            .map_err(|err| err.to_string())
    }
}
