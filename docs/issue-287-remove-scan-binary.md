# Issue #287 — Remove the Deprecated `scan` Binary

## Problem

`src/bin/scan.rs` is a leftover from before the `depradar` CLI existed. It does the same job as two `depradar` subcommands:

| Legacy | Replacement |
|---|---|
| `scan <package-name>` | `depradar name <package-name>` |
| `scan <lockfile>` | `depradar scan <lockfile>` |

It has printed `warning: \`scan\` is deprecated; …` to stderr for a while. Keeping it means a third binary to build and ship, plus a second code path for ignore-file loading that can drift from the `depradar` one.

## Goal

Delete the binary and everything that only exists because of it, so `cargo build` produces exactly two binaries: `depradar` and `osv`.

## Affected Files

| File | Change |
|---|---|
| `src/bin/scan.rs` | Delete |
| `tests/cli.rs` | Remove `name_output_matches_the_legacy_scan_binary` (uses `Command::cargo_bin("scan")`) |
| `CONTRIBUTING.md` | `cargo run --bin scan -- expresss` → `cargo run --bin depradar -- name expresss` |
| `README.md` | Line ~304 mentions `cargo run --bin scan`; reword to `depradar name` |
| `PROGRESS.md` | Add entry noting the removal |

Leave the historical PROGRESS.md entry (line ~371) and the old `docs/issue-24-*.md` design note alone. They describe what happened at the time and aren't usage instructions.

## Design

### Binaries are discovered automatically

`Cargo.toml` has no `[[bin]]` sections. Cargo picks up every file in `src/bin/`, so deleting `src/bin/scan.rs` is enough to drop the target. No manifest edit is needed.

### Replacing the legacy-parity test

`name_output_matches_the_legacy_scan_binary` checks that `depradar name expresss` and `scan expresss` print the same thing. Once `scan` is gone there's nothing to compare against. Remove the test, but keep its useful assertion by checking that the `depradar name` output still contains `likely typosquat`, if no other test already covers that:

```rust
#[test]
fn name_flags_a_known_typosquat() {
    let output = depradar().args(["name", "expresss"]).output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("likely typosquat"));
}
```

### Check nothing else builds or runs `scan`

Before opening the PR, run:

```bash
grep -rn 'bin scan\|bin/scan\|cargo_bin("scan")\|target/release/scan' \
  --include=*.rs --include=*.md --include=*.toml --include=*.yml --include=*.yaml . \
  | grep -v target/
```

Pay particular attention to `.github/workflows/` and any release packaging script that lists binaries by name. Update those too.

## Acceptance Criteria Mapping

### `cargo build` produces only `depradar` and `osv`

```bash
cargo build
ls target/debug/ | grep -E '^(depradar|osv|scan)$'
# expected:
# depradar
# osv
```

If you want this checked automatically, `cargo metadata --format-version 1 --no-deps` lists every target. Asserting that the `bin` targets are exactly `{depradar, osv}` makes a cheap test, but it isn't required.

### CI

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Implementation Checklist

- [ ] `git rm src/bin/scan.rs`
- [ ] Remove or replace `name_output_matches_the_legacy_scan_binary` in `tests/cli.rs`
- [ ] Update `CONTRIBUTING.md` dev-setup command
- [ ] Update the `README.md` mention
- [ ] Grep workflows and scripts for the `scan` binary name
- [ ] Confirm `target/debug/` has only `depradar` and `osv`
- [ ] PROGRESS.md entry stating the binary was removed and what replaces it
- [ ] PR description includes `Closes #287`

## Notes

- This is a breaking change for anyone with `scan` in a script. Say so in the PROGRESS entry (it doubles as the changelog) and give the replacement commands from the table above.
- Don't confuse the binary with the `depradar scan` **subcommand**. The subcommand stays, and every other `"scan"` in `tests/cli.rs` refers to it.
