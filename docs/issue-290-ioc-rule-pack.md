# Issue #290 — RepoGuard Day 2: Indicator-of-Compromise Rule Pack in `data/ioc/`

Part of the [RepoGuard roadmap](../ROADMAP.md), Phase 1, Day 2.

> **Depends on #289.** This builds on the `repo-scan` subcommand, `RepoFinding` and the `Rule` trait from Day 1 ([design](issue-289-repo-scan-subcommand.md)). Check #289 has merged before starting.

## Problem

After Day 1, `depradar repo-scan` walks a repository but has no rules, so it never reports anything. The quickest useful detection is matching exact, published indicators of compromise for known campaigns: strings, file names and package names. That kind of rule rarely produces false positives, and it's how security teams usually share what they've learned about a campaign.

## Goal

1. A data-only rule file format, one file per campaign, kept in `data/ioc/` and embedded at compile time.
2. A loader that parses and validates it.
3. A `Rule` implementation that flags matches, tagged with the campaign name.
4. PolinRider as the first pack.

## Affected Files

| File | Change |
|---|---|
| `data/ioc/README.md` (new) | Format reference and how to add a campaign |
| `data/ioc/polinrider.toml` (new) | First rule pack |
| `src/repo/ioc.rs` (new) | `IocPack` types, loader, `IocRule: Rule` |
| `src/repo/mod.rs` | Register the embedded packs in the default rule set |
| `tests/fixtures/repo-scan/ioc/` (new) | Positive and negative fixtures |
| `tests/repo_scan_ioc.rs` (new) | Loader and detection tests |
| `PROGRESS.md` | Add entry |

## Design

### Format: TOML

Use TOML rather than JSON. The project already uses it for `depradar.toml` and has the `toml` crate, and TOML allows comments, which rule authors need for citing sources and explaining why a marker is specific. Load it with `serde` and `#[serde(deny_unknown_fields)]`, the same way `ProjectConfig` is loaded, so a typo in a rule file fails loudly.

```toml
# data/ioc/<campaign>.toml
schema = 1

[campaign]
id = "polinrider"                    # stable, lowercase, used in rule ids
name = "PolinRider"                  # shown in findings
severity = "critical"                # default for every indicator in the pack
references = [
  "https://…advisory…",              # where each indicator came from
]

# Exact substrings found inside file contents.
[[markers]]
id = "loader-var"                    # rule id becomes ioc/polinrider/loader-var
value = "…"                          # literal; no regex on Day 2
files = ["*.config.js", "*.config.mjs", "*.config.cjs", "*.config.ts"]  # optional glob filter
description = "Variable name used by the PolinRider loader"

# File paths whose presence is itself suspicious.
[[file_names]]
id = "fake-font"
glob = "**/fonts/*.woff2.js"
description = "Payload disguised as a font file"

# Package names referenced from package.json scripts or dependencies.
[[packages]]
id = "dropper-pkg"
ecosystem = "npm"
name = "…"
description = "Package installed by the payload's postinstall step"
```

Keep Day 2 to **literal** matching (substring, glob, exact name). Regexes make rules more expressive but also more prone to false positives and catastrophic backtracking. Add them later only if a real indicator needs one.

### Loader

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IocPack {
    pub schema: u32,
    pub campaign: Campaign,
    #[serde(default)] pub markers: Vec<Marker>,
    #[serde(default)] pub file_names: Vec<FileName>,
    #[serde(default)] pub packages: Vec<PackageIoc>,
}

/// Packs compiled into the binary.
const EMBEDDED: &[(&str, &str)] = &[
    ("polinrider.toml", include_str!("../../data/ioc/polinrider.toml")),
];

pub fn embedded_packs() -> &'static [IocPack]  // parsed once via OnceLock, like the typosquat dataset (#22)
pub fn parse_pack(source: &str) -> Result<IocPack, IocError>
```

Validation in `parse_pack`, beyond what serde checks:

- `schema == 1`
- `campaign.id` matches `^[a-z0-9-]+$`
- indicator `id`s are unique within the pack
- `value` / `glob` / `name` are non-empty
- markers are at least ~8 bytes long, so a vague string doesn't flag half the internet

Use `include_str!`, not runtime file reads. The binary should work from any directory, and the roadmap's later "community IOC feed" (Day 26) is the point where runtime loading comes in.

Add a unit test that every entry in `EMBEDDED` parses. That catches a broken pack at `cargo test`, before it ships.

### Matching (`IocRule: Rule`)

| Indicator | How it matches | `RepoFinding.line` |
|---|---|---|
| `markers` | Byte substring search in file contents, restricted to `files` globs when set | Line of the first match |
| `file_names` | Glob against the path relative to the repo root | `None` (whole file) |
| `packages` | Parse `package.json` files (reuse `lockfile::npm::parse_package_json`), compare names. Also look for the name in `scripts` values | Line naming the package (see `line_of` in `report/sarif.rs`) |

Every finding sets `campaign: Some("PolinRider")`, `rule_id: "ioc/polinrider/<indicator-id>"` and the pack's severity. The message reads like `PolinRider indicator: <description>`.

Report one finding per (file, indicator), not one per occurrence. A minified file containing a marker 300 times should be one alert.

For SARIF, emit one `rule` per indicator, with `properties.tags = ["security", "malware", "ioc", "<campaign-id>"]`, so GitHub code scanning can filter by campaign.

### PolinRider pack contents

Take indicators **only from published, citable sources** (the advisories referenced in ROADMAP.md and the issue thread), and put each source URL in `references`. Don't make up markers. A wrong IOC is worse than a missing one. Cover the three techniques ROADMAP.md lists:

1. Payloads appended to build configs (`postcss.config.*`, `tailwind.config.*`, …): content `markers` scoped with `files`.
2. Auto-run hooks in `.vscode/tasks.json` and install scripts in `package.json`: `markers` and `packages`.
3. Payloads disguised as font files: `file_names`.

If an indicator is too generic to stand alone (e.g. "long whitespace run"), leave it out of the IOC pack. It belongs in a heuristic rule on a later roadmap day.

## Fixtures (defanged)

Fixtures go under `tests/fixtures/repo-scan/ioc/polinrider/`.

- **Positive:** one small file per indicator. Each contains the marker string, sits at the flagged path, or lists the package name, inside an otherwise harmless file. For example, a `postcss.config.js` that exports `{}` and has the marker in a **comment** or an unused string literal. No executable payload, no real URLs or C2 hosts (use `example.invalid`), and nothing that would run on `npm install`.
- **Negative:** near-misses that must **not** be flagged:
  - a real-looking `postcss.config.js` and `tailwind.config.js`
  - a real `.woff2` font under `fonts/`
  - a `package.json` that depends on a package whose name merely *contains* an IOC package name
  - a marker string in a file outside the `files` glob, if the marker is scoped

Add a short `tests/fixtures/repo-scan/ioc/README.md` saying the fixtures are defanged and why they contain these strings, so a reviewer or an antivirus alert doesn't mistake them for real malware.

Remember the #289 self-scan test: DepRadar's own tree must still produce an empty SARIF. Make sure the exclusion of `tests/fixtures/**` (agreed in #289) covers these files, and that `data/ioc/polinrider.toml` itself doesn't match its own markers. Either exclude `data/ioc/**` from the default walk or skip `.toml` files in `data/ioc/`.

## Acceptance Criteria Mapping

### Every listed marker flags the fixture with the campaign name

Generate the test from the pack itself, so adding an indicator without a fixture fails the build:

```rust
#[test]
fn every_polinrider_indicator_is_flagged_with_campaign_name() {
    let pack = embedded_packs().iter().find(|p| p.campaign.id == "polinrider").unwrap();
    let report = scan_repo(&fixture("ioc/polinrider/positive"), &[], &default_rules()).unwrap();

    for id in pack.indicator_ids() {
        let rule_id = format!("ioc/polinrider/{id}");
        let hit = report.findings.iter().find(|f| f.rule_id == rule_id)
            .unwrap_or_else(|| panic!("no fixture triggers {rule_id}"));
        assert_eq!(hit.campaign.as_deref(), Some("PolinRider"));
    }
}

#[test]
fn polinrider_negative_fixtures_are_clean() {
    let report = scan_repo(&fixture("ioc/polinrider/negative"), &[], &default_rules()).unwrap();
    assert!(report.findings.is_empty(), "{:#?}", report.findings);
}
```

Also add a CLI-level test: `depradar --format json repo-scan <positive>` exits non-zero, and the JSON contains `"campaign": "PolinRider"`.

### Loader tests

- Every embedded pack parses.
- Unknown field → error naming the field.
- Duplicate indicator id → error.
- `schema = 2` → error.

### CI

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Implementation Checklist

- [ ] Confirm #289 is merged; rebase on `main`
- [ ] `data/ioc/README.md` documenting the format
- [ ] `src/repo/ioc.rs`: types, `parse_pack`, validation, `embedded_packs`, `IocRule`
- [ ] Register `IocRule` in the default `repo-scan` rule set
- [ ] `data/ioc/polinrider.toml` with a cited reference for every indicator
- [ ] Defanged positive fixture per indicator, plus negative near-misses
- [ ] Pack-driven "every indicator is flagged" test and negative test
- [ ] Loader error tests
- [ ] Self-scan of DepRadar still yields empty SARIF
- [ ] PROGRESS.md entry, PR description includes `Closes #290`

## Notes

- **Adding a campaign later** should mean adding one `.toml` file, one line in `EMBEDDED`, and fixtures, with no code changes. If it needs more than that, the format is missing something.
- Store marker `value`s literally, not hashed or encoded. Reviewers have to be able to read what's being matched.
- The roadmap's Day 26 replaces `EMBEDDED` with a signed, updatable feed. Keep `parse_pack` independent of where the text comes from so it can be reused there.
