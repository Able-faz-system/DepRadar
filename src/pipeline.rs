//! End-to-end scan of a project lockfile: parse it, run the typosquat scorer
//! over every package name, check every pinned version against OSV, and
//! collect the results into one [`Report`].

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::cve::osv_client::{query_osv_batch, OsvBatchQuery};
use crate::lockfile::{parse_lockfile, LockfileError};
use crate::model::{Dependency, Ecosystem, Finding, FindingKind};
use crate::typosquat::scanner::{score_package, ScanResult};

/// Error type an [`AdvisorySource`] can fail with.
pub type AdvisoryError = Box<dyn Error + Send + Sync>;

/// Where known-vulnerability data comes from. [`OsvAdvisories`] queries the
/// live OSV API; tests substitute their own.
pub trait AdvisorySource {
    /// Returns, for each query in order, the IDs of the advisories affecting
    /// that exact package version (empty if none).
    fn query(&self, queries: &[OsvBatchQuery<'_>]) -> Result<Vec<Vec<String>>, AdvisoryError>;
}

/// [`AdvisorySource`] backed by OSV's batch API ([`query_osv_batch`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct OsvAdvisories;

impl AdvisorySource for OsvAdvisories {
    fn query(&self, queries: &[OsvBatchQuery<'_>]) -> Result<Vec<Vec<String>>, AdvisoryError> {
        let results = query_osv_batch(queries)?;
        Ok(results
            .into_iter()
            .map(|result| result.vulns.into_iter().map(|vuln| vuln.id).collect())
            .collect())
    }
}

/// What [`scan_lockfile`] checks.
#[derive(Clone, Copy)]
pub struct ScanOptions<'a> {
    /// Score every package name with [`score_package`].
    pub typosquat: bool,
    /// Where to look up known vulnerabilities, or `None` to skip CVE checks.
    pub advisories: Option<&'a dyn AdvisorySource>,
}

impl Default for ScanOptions<'_> {
    fn default() -> Self {
        ScanOptions {
            typosquat: true,
            advisories: Some(&OsvAdvisories),
        }
    }
}

/// Totals for a [`Report`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Distinct `(name, version)` pairs in the lockfile.
    pub packages: usize,
    /// How many of those had a pinned version and were checked against the
    /// advisory source.
    pub queried: usize,
    pub typosquats: usize,
    pub vulnerabilities: usize,
    pub warnings: usize,
}

/// The outcome of scanning one lockfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub path: PathBuf,
    pub ecosystem: Ecosystem,
    pub findings: Vec<Finding>,
    /// Problems parsing the lockfile. These are not findings: they mean part
    /// of the file couldn't be checked.
    pub warnings: Vec<String>,
    pub summary: Summary,
}

impl Report {
    /// True if any finding was reported.
    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Scanned {} ({} packages, {} pinned versions checked for known vulnerabilities)",
            self.path.display(),
            self.summary.packages,
            self.summary.queried
        )?;

        if self.findings.is_empty() {
            writeln!(f, "\nNo findings.")?;
        } else {
            writeln!(f, "\nFindings ({}):", self.findings.len())?;
            for finding in &self.findings {
                writeln!(f, "  - {finding}")?;
            }
        }

        if !self.warnings.is_empty() {
            writeln!(f, "\nWarnings ({}):", self.warnings.len())?;
            for warning in &self.warnings {
                writeln!(f, "  - {warning}")?;
            }
        }

        write!(
            f,
            "\nSummary: {} typosquat(s), {} vulnerability finding(s), {} warning(s)",
            self.summary.typosquats, self.summary.vulnerabilities, self.summary.warnings
        )
    }
}

/// Errors that stop a scan from producing a report.
#[derive(Debug)]
pub enum ScanError {
    Lockfile(LockfileError),
    Advisories(AdvisoryError),
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::Lockfile(err) => write!(f, "{err}"),
            ScanError::Advisories(err) => write!(f, "vulnerability lookup failed: {err}"),
        }
    }
}

impl Error for ScanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ScanError::Lockfile(err) => Some(err),
            ScanError::Advisories(err) => Some(err.as_ref()),
        }
    }
}

/// Scans the lockfile at `path` for typosquats and known vulnerabilities.
///
/// Repeated `(name, version)` pairs (the same package installed in several
/// places) are checked once, and all pinned versions go to the advisory
/// source in a single batch.
pub fn scan_lockfile(
    path: impl AsRef<Path>,
    opts: &ScanOptions<'_>,
) -> Result<Report, ScanError> {
    let path = path.as_ref();
    let parsed = parse_lockfile(path).map_err(ScanError::Lockfile)?;
    let packages = dedupe(parsed.dependencies);

    let mut findings = Vec::new();

    if opts.typosquat {
        let mut scores: HashMap<&str, ScanResult> = HashMap::new();
        for dependency in &packages {
            let result = scores
                .entry(dependency.name.as_str())
                .or_insert_with(|| score_package(&dependency.name));
            if result.is_likely_typosquat() {
                findings.push(finding(
                    dependency,
                    FindingKind::Typosquat {
                        distance_match: result.distance_match.clone(),
                        homoglyph_match: result.homoglyph_match.clone(),
                    },
                ));
            }
        }
    }

    let mut queried = 0;
    if let Some(source) = opts.advisories {
        let pinned: Vec<(&Dependency, &str)> = packages
            .iter()
            .filter_map(|dependency| Some((dependency, dependency.version.as_deref()?)))
            .collect();
        let queries: Vec<OsvBatchQuery<'_>> = pinned
            .iter()
            .map(|&(dependency, version)| OsvBatchQuery {
                name: &dependency.name,
                ecosystem: dependency.ecosystem,
                version,
            })
            .collect();

        if !queries.is_empty() {
            let results = source.query(&queries).map_err(ScanError::Advisories)?;
            queried = queries.len();
            for ((dependency, _), advisory_ids) in pinned.iter().zip(results) {
                for advisory_id in advisory_ids {
                    findings.push(finding(
                        dependency,
                        FindingKind::Vulnerability { advisory_id },
                    ));
                }
            }
        }
    }

    let count = |is_kind: fn(&FindingKind) -> bool| {
        findings.iter().filter(|f| is_kind(&f.kind)).count()
    };
    let summary = Summary {
        packages: packages.len(),
        queried,
        typosquats: count(|kind| matches!(kind, FindingKind::Typosquat { .. })),
        vulnerabilities: count(|kind| matches!(kind, FindingKind::Vulnerability { .. })),
        warnings: parsed.warnings.len(),
    };

    Ok(Report {
        path: path.to_path_buf(),
        ecosystem: parsed.ecosystem,
        findings,
        warnings: parsed.warnings,
        summary,
    })
}

/// Collapses repeated `(name, version)` pairs, keeping first-seen order. A
/// pair counts as direct if any of its installs is direct.
fn dedupe(dependencies: Vec<Dependency>) -> Vec<Dependency> {
    let mut index: HashMap<(String, Option<String>), usize> = HashMap::new();
    let mut unique: Vec<Dependency> = Vec::new();
    for dependency in dependencies {
        let key = (dependency.name.clone(), dependency.version.clone());
        match index.get(&key) {
            Some(&existing) => unique[existing].direct |= dependency.direct,
            None => {
                index.insert(key, unique.len());
                unique.push(dependency);
            }
        }
    }
    unique
}

fn finding(dependency: &Dependency, kind: FindingKind) -> Finding {
    Finding {
        package: dependency.name.clone(),
        version: dependency.version.clone(),
        ecosystem: dependency.ecosystem,
        direct: dependency.direct,
        kind,
    }
}
