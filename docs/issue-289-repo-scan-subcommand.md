# Issue #289 — RepoGuard Day 1: `depradar repo-scan [PATH]`

Part of the [RepoGuard roadmap](../ROADMAP.md), Phase 1, Day 1.

## Problem

Every existing `depradar` mode starts from a lockfile. The PolinRider campaign (see ROADMAP.md) hid its payload in the repository's own files: build configs, `.vscode/tasks.json`, `package.json` scripts and fake font files. None of those show up in a dependency graph, so DepRadar can't see them today.

## Goal

Add a `repo-scan` subcommand that walks a repository's files and emits findings through the existing report formats. Day 1 only builds the plumbing: the walker, the finding type and the reporters. Detection rules come in later days (#290 adds the first rule pack), so on a clean repo this subcommand reports nothing, and it should do that correctly.

## Affected Files

| File | Change |
|---|---|
| `src/repo/mod.rs` (new) | `walk()`, `RepoFinding`, `RepoReport`, and a `Rule` trait for later days |
| `src/lib.rs` | `pub mod repo;` |
| `src/bin/depradar.rs` | `Command::RepoScan { path, exclude }`; add `Format::Sarif` |
| `src/config.rs` | `ConfigFormat::Sarif` so `format = "sarif"` works in `depradar.toml` |
| `src/report/sarif.rs` | A `repo_to_sarif(&RepoReport)` alongside `to_sarif` |
| `Cargo.toml` | Add `ignore` (for `.gitignore`) and possibly `globset` |
| `tests/repo_scan.rs` (new) | CLI and walker tests |
| `tests/fixtures/repo-scan/` (new) | Positive and negative fixture trees |
| `tests/snapshots/depradar-help.txt` | Regenerate (new subcommand) |
| `PROGRESS.md` | Add entry |

## Design

### What exists today and what's missing

- **Walking:** `pipeline::scan_dir` does a hand-rolled `read_dir` walk that skips `SKIPPED_DIRS` (`node_modules`, `.git`, `target`, `dist`). It does **not** read `.gitignore`.
- **Exclude globs:** `is_excluded`/`glob_match` in `src/bin/depradar.rs` is a minimal matcher that lives in the binary, not the library.
- **Report formats:** the CLI `Format` enum has only `Text` and `Json`. `report::sarif::to_sarif` exists, but it takes a lockfile `Report` of package `Finding`s and is only called from tests. There is no `--format sarif` yet.

So Day 1 needs a file-level finding type, a SARIF path for it, and a `.gitignore`-aware walker.

### Walker: use the `ignore` crate

[`ignore`](https://crates.io/crates/ignore), from the ripgrep author, handles `.gitignore`, `.ignore`, `.git/info/exclude`, nested ignore files and hidden files. Hand-writing that correctly is hard, so don't.

```rust
pub fn walk(root: &Path, exclude: &[String]) -> Result<Vec<PathBuf>, RepoScanError> {
    let mut overrides = ignore::overrides::OverrideBuilder::new(root);
    for pattern in exclude {
        overrides.add(&format!("!{pattern}"))?; // `!` = exclude in override syntax
    }
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)          // we DO want .vscode/, .github/ etc.
        .git_ignore(true)
        .require_git(false)     // honour .gitignore even outside a git checkout
        .overrides(overrides.build()?)
        .filter_entry(|e| e.file_name() != ".git")
        .build();
    // collect files, paths relative to root, sorted for stable output
}
```

Notes:

- `hidden(false)` matters. Attack surface like `.vscode/tasks.json` and `.github/workflows/` lives in dotfiles, and the default skips them.
- Always skip `.git/`. Git history is a later roadmap day, not raw object files.
- Keep `node_modules/` skipped by default even without a `.gitignore`. Reuse `SKIPPED_DIRS`.
- Don't follow symlinks (matches `scan_dir`).
- Sort the output so reports are deterministic.

**Exclude semantics.** `depradar.toml`'s `exclude` doc says "Paths are matched relative to the scanned root". Both `ignore`'s overrides and `globset` use gitignore-style globs, which differ slightly from the existing `glob_match` (e.g. a bare `*.min.js` matches at any depth). Document whichever behaviour you ship in the `--exclude` help text. Ideally move the matcher into the library so `scan` and `repo-scan` agree.

### Finding type

The existing `Finding` is about a package (`package`, `version`, `ecosystem`, `path` as a dependency chain), which doesn't fit "line 1 of `postcss.config.js` has a 400-column whitespace run". Add a separate type:

```rust
/// One suspicious thing in one repository file.
pub struct RepoFinding {
    /// Stable rule id, used as the SARIF `ruleId`, e.g. `ioc/polinrider/marker`.
    pub rule_id: String,
    pub severity: Severity,
    /// Path relative to the scanned root, forward slashes.
    pub file: PathBuf,
    /// 1-based line, if the rule can point at one.
    pub line: Option<usize>,
    pub message: String,
    /// Campaign name, when the rule belongs to a known campaign (Day 2+).
    pub campaign: Option<String>,
}

pub struct RepoReport {
    pub root: PathBuf,
    pub files_scanned: usize,
    pub findings: Vec<RepoFinding>,
}
```

Add a small `Rule` trait now so #290 can plug in without touching the walker:

```rust
pub trait Rule {
    fn check(&self, path: &Path, contents: &[u8]) -> Vec<RepoFinding>;
}

pub fn scan_repo(root: &Path, exclude: &[String], rules: &[&dyn Rule]) -> Result<RepoReport, RepoScanError>;
```

With zero rules registered, `scan_repo` returns an empty report, which is the Day 1 acceptance case. Cap the file size you read (e.g. skip files over 5 MiB) and read bytes, not `String`, since repos contain binaries.

### Reporters

- **Text:** one line per finding, `file:line: [rule_id] message`, then a summary (`N files scanned, M findings`). Follow `report::text` conventions (colour, `--quiet`).
- **JSON:** one document on stdout, matching the `--format json` contract (`stdout_json` in `tests/cli.rs` checks for exactly one document).
- **SARIF:** add `repo_to_sarif(&RepoReport) -> Value` in `src/report/sarif.rs`. Reuse `SARIF_SCHEMA`, `SARIF_VERSION`, `INFORMATION_URI`, `level()` and `security_severity()`. Results get `physicalLocation.artifactLocation.uri = file` and `region.startLine = line.unwrap_or(1)`. With no findings, emit `"results": []` and an empty `rules` array. Both are valid SARIF.

Add `Format::Sarif` to the CLI and `ConfigFormat::Sarif` to config. For the existing `scan` subcommand you can either wire `--format sarif` through `to_sarif` (a nice bonus, since it already exists) or fail with a clear "sarif is only supported by repo-scan for now". Don't silently fall back to text.

### CLI

```rust
/// Scan a repository's own files (not its lockfiles) for malware markers
RepoScan {
    /// Repository root. Defaults to the current directory.
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Glob patterns for paths to skip (may be repeated). Overrides
    /// `exclude` in `depradar.toml`.
    #[arg(long = "exclude", value_name = "GLOB")]
    exclude: Vec<String>,
},
```

Load `depradar.toml` from `path` the same way `scan` does (`ResolvedConfig`), so `format`, `fail_on` and `exclude` all apply. Exit codes match `scan`: `0` for clean or below `--fail-on`, `1` for findings at or above `--fail-on`, `2` for usage or IO errors.

## Fixtures

Fixtures go under `tests/fixtures/repo-scan/`. Following the roadmap principle, they must be **defanged**: real structure, harmless payload.

| Fixture | Purpose |
|---|---|
| `clean/` | A small normal project (`package.json`, `src/index.js`, `.vscode/settings.json`). Must produce zero findings. |
| `gitignored/` | Contains `.gitignore` listing `ignored.js`, plus `ignored.js`. Assert `ignored.js` is never walked. |
| `excluded/` | `depradar.toml` with `exclude = ["vendor/**"]` and a `vendor/` file. Assert it's skipped, and that `--exclude` overrides the file. |
| `positive/` | A file that a test-only `Rule` flags (e.g. a rule matching the literal `DEPRADAR-TEST-MARKER`). This proves findings reach every reporter without waiting for real rules. |

Put the test-only rule in the test file, not the library. Note that `.gitignore` files inside `tests/fixtures/` are real ignore files, and git honours them for the DepRadar repo itself. Commit the ignored fixture file with `git add -f`, and check that `git status` is clean afterwards.

## Acceptance Criteria Mapping

### Running on DepRadar itself → empty, valid SARIF

```rust
#[test]
fn repo_scan_of_depradar_itself_is_empty_valid_sarif() {
    let output = depradar()
        .args(["--format", "sarif", "repo-scan", env!("CARGO_MANIFEST_DIR")])
        .output().unwrap();
    assert!(output.status.success());
    let sarif: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(sarif["runs"][0]["results"], json!([]));
    // validate against tests/fixtures/sarif-schema-2.1.0.json, as tests/sarif.rs does
}
```

This test will also stop later rule packs from flagging DepRadar's own fixtures. Those fixtures live under `tests/fixtures/`, so either have a `depradar.toml` at the repo root exclude `tests/fixtures/**`, or have the test pass `--exclude tests/fixtures/**`. Pick one and document it, because #290's IOC fixtures will contain every marker on purpose.

### Positive and negative fixtures

See the Fixtures section above. Run each fixture through all three formats.

### CI

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check licenses bans sources   # new dependency: `ignore` is MIT/Unlicense
```

## Implementation Checklist

- [ ] Add the `ignore` dependency and confirm `cargo deny` passes
- [ ] `src/repo/mod.rs` with `walk`, `RepoFinding`, `RepoReport`, `Rule`, `scan_repo`
- [ ] `Format::Sarif` and `ConfigFormat::Sarif`
- [ ] `repo_to_sarif` in `src/report/sarif.rs`
- [ ] `Command::RepoScan` wired up with config loading and exit codes
- [ ] Fixtures: clean, gitignored, excluded, positive (all defanged)
- [ ] Self-scan SARIF test validated against the bundled schema
- [ ] Regenerate `tests/snapshots/depradar-help.txt`
- [ ] README: short `repo-scan` usage section
- [ ] PROGRESS.md entry, PR description includes `Closes #289`

## Notes

- Keep Day 1 small: no detection logic. Reviewers should be able to read the PR as "new walker + new output path".
- Reusing `src/behavioral/` detectors (obfuscation, scripts, binaries) on repo files is a later roadmap day. The `Rule` trait is the hook for that.
- `#290` depends on this PR. Agree on the `Rule` trait and `RepoFinding.campaign` field here so it doesn't need to change them.
