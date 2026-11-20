# repo-scan (RepoGuard) — implementation notes

Design notes for the `[repo-scan]` issues on the RepoGuard roadmap. Each one
says what the issue asks for, how we suggest building it, which fixtures it
needs, and which edge cases we already know about. The issue is the
source of truth for acceptance criteria. These notes cover the "how".

| Day | Issue | Doc |
|---|---|---|
| 3 | [#291 Whitespace-padding detector](https://github.com/DepRadarOrg/DepRadar/issues/291) | [day-03-whitespace-padding.md](day-03-whitespace-padding.md) |
| 4 | [#292 Obfuscation over repo source](https://github.com/DepRadarOrg/DepRadar/issues/292) | [day-04-source-obfuscation.md](day-04-source-obfuscation.md) |
| 5 | [#293 Auto-run trap detector](https://github.com/DepRadarOrg/DepRadar/issues/293) | [day-05-auto-run-traps.md](day-05-auto-run-traps.md) |
| 6 | [#294 Lifecycle scripts in the repo's `package.json`](https://github.com/DepRadarOrg/DepRadar/issues/294) | [day-06-lifecycle-scripts.md](day-06-lifecycle-scripts.md) |
| 7 | [#295 Disguised-file detector](https://github.com/DepRadarOrg/DepRadar/issues/295) | [day-07-disguised-files.md](day-07-disguised-files.md) |
| 9 | [#297 Forged-merge detector](https://github.com/DepRadarOrg/DepRadar/issues/297) | [day-09-forged-merge.md](day-09-forged-merge.md) |
| 10 | [#298 Evil-merge detector](https://github.com/DepRadarOrg/DepRadar/issues/298) | [day-10-evil-merge.md](day-10-evil-merge.md) |

## Before you start

Each day builds on the one before it. **Check that the previous day's PR
has merged before you begin.** If that work landed differently from what
these notes assume (a different module path, `Finding` type, or git
access layer), follow the code that actually landed. These notes yield to it.

When these notes were written, none of the earlier days were on `main`.
That includes `src/behavioral/binaries.rs`, which Day 7 references. Names
like `src/repo_scan/` and `Finding` below are placeholders for whatever
Days 1–6 establish.

## Conventions shared by every day

- **Fixtures are defanged.** No live malware, no real payloads, no real
  C2 URLs. A "malicious" fixture only needs the *shape* of the attack: for
  example, a `.woff2` file whose bytes are `console.log("defanged")`. Use
  `example.invalid` for any URL.
- **Every detector gets positive and negative fixtures.** The negative
  fixtures matter just as much. Each detector below has a known
  legitimate case that looks like an attack, and that case must have a
  test proving it is *not* flagged.
- **Build git fixtures at test time.** Don't commit them. A nested
  `.git/` directory can't be committed to this repo. Tests should create
  a temporary repository, write the commits they need, and run the
  detector on it. Put the helper (`init`, `commit`, `merge`, a fixed
  author and date for determinism) in one shared test-support module, so
  Days 9 and 10 reuse it.
- **Keep the self-scan clean.** Day 1 requires `depradar repo-scan` on
  this repo to produce an empty SARIF. Positive fixtures for Days 3–7
  will trip their own detectors. The same goes for the existing
  `tests/fixtures/behavioral/` packages and the committed
  `.claude/settings.json` (see Day 5). Pick one approach in the first PR
  that hits this: exclude `tests/fixtures/**` in the repo's own
  `depradar.toml`, or build the positive fixtures inside the tests. Every
  later day then follows it.
- **Offline by default.** Any network check, such as Day 9's GitHub API
  confirmation, is optional. Its tests skip when the network is
  unavailable, the same way `src/cve/` already handles OSV.
- **Definition of Done** (from every issue):
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --locked`
  - A [PROGRESS.md](../../PROGRESS.md) entry (What / Tests / By / Closes / Status)
  - `Closes #<issue>` in the PR description
