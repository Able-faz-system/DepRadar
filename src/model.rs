//! Common shapes shared by every scanner, lockfile parser and report.

use serde::Serialize;

use crate::cve::osv_client::{Ecosystem, OsvVulnerability};
use crate::typosquat::scanner::ScanResult;

/// A resolved package a project depends on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    /// False for dev-only dependencies.
    pub direct: bool,
}

/// The kind of signal a [`Finding`] was raised for. Extend as scanners land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FindingKind {
    Typosquat,
    KnownVulnerability,
    InstallScript,
    Obfuscation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Unknown,
    Low,
    Medium,
    High,
    Critical,
}

/// One scanner result about one dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub dependency: Dependency,
    pub kind: FindingKind,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub references: Vec<String>,
}

impl From<ScanResult> for Finding {
    /// Builds a typosquat finding. The scan only knows the candidate name,
    /// so the version is left empty and the ecosystem is npm (the only one
    /// the popular-package dataset covers).
    fn from(result: ScanResult) -> Self {
        let severity = if result.homoglyph_match.is_some() {
            Severity::High
        } else if result.distance_match.is_some() {
            Severity::Medium
        } else {
            Severity::Unknown
        };
        let mut matches = Vec::new();
        if let Some((name, distance)) = &result.distance_match {
            matches.push(format!(
                "edit distance {distance} from popular package \"{name}\""
            ));
        }
        if let Some(name) = &result.homoglyph_match {
            matches.push(format!(
                "homoglyph impersonation of popular package \"{name}\""
            ));
        }
        Finding {
            title: format!("{} looks like a typosquat", result.candidate),
            detail: matches.join("; "),
            dependency: Dependency {
                name: result.candidate,
                version: String::new(),
                ecosystem: Ecosystem::Npm,
                direct: true,
            },
            kind: FindingKind::Typosquat,
            severity,
            references: Vec::new(),
        }
    }
}

/// Builds a known-vulnerability finding for the dependency an OSV record
/// was returned for.
impl From<(Dependency, OsvVulnerability)> for Finding {
    fn from((dependency, vuln): (Dependency, OsvVulnerability)) -> Self {
        Finding {
            dependency,
            kind: FindingKind::KnownVulnerability,
            severity: Severity::Unknown,
            title: vuln.summary.unwrap_or(vuln.id),
            detail: vuln.details.unwrap_or_default(),
            references: vuln.references.into_iter().map(|r| r.url).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typosquat::scanner::score_package;

    #[test]
    fn scan_result_converts_to_typosquat_finding() {
        let finding = Finding::from(score_package("lodahs"));
        assert_eq!(finding.kind, FindingKind::Typosquat);
        assert_eq!(finding.severity, Severity::Medium);
        assert_eq!(finding.dependency.name, "lodahs");
        assert!(finding.detail.contains("lodash"));
    }

    #[test]
    fn osv_vulnerability_converts_to_known_vulnerability_finding() {
        let vuln: OsvVulnerability = serde_json::from_str(
            r#"{"id":"GHSA-x","summary":"ReDoS","references":[{"type":"WEB","url":"https://e.x/a"}]}"#,
        )
        .unwrap();
        let dep = Dependency {
            name: "lodash".into(),
            version: "4.17.4".into(),
            ecosystem: Ecosystem::Npm,
            direct: true,
        };
        let finding = Finding::from((dep.clone(), vuln));
        assert_eq!(finding.kind, FindingKind::KnownVulnerability);
        assert_eq!(finding.dependency, dep);
        assert_eq!(finding.title, "ReDoS");
        assert_eq!(finding.references, vec!["https://e.x/a"]);
        assert!(serde_json::to_string(&finding).is_ok());
    }
}
