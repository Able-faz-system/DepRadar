# Issue #286 — Replace `cargo run --bin scan` with `depradar` in CONTRIBUTING

## Problem

`src/bin/scan.rs` is deprecated. Its first line prints `warning: \`scan\` is
deprecated; use \`depradar name <package>\` or \`depradar scan <path>\`
instead`, but [CONTRIBUTING.md](../CONTRIBUTING.md) still points new
contributors at it:

```bash
# Try the typosquat scanner
cargo run --bin scan -- expresss
```

The first command a newcomer copies from the setup guide prints a
deprecation warning.

## Goal

Every doc example runs the `depradar` binary (`src/bin/depradar.rs`) with
its `name` and `scan` subcommands, and no doc mentions `--bin scan`.

## Affected Files

`grep -rn "bin scan" --include=*.md .` currently finds four files:

| File | Line(s) | Change |
|---|---|---|
| `CONTRIBUTING.md` | 54 | Replace with `depradar name` + `depradar scan` examples (the change the issue asks for) |
| `README.md` | 304 | History table row for #11. Reword to say the PR added the CLI that is now `depradar name`, or leave it as history (see Open Questions) |
| `docs/issue-24-ecosystem-scoped-scoring.md` | 160–162 | Replace the examples (see note below) |
| `PROGRESS.md` | 371 | Historical entry for #11 (see Open Questions) |
| `PROGRESS.md` | new | Add an entry for #286 |

## Change

### CONTRIBUTING.md

Replace lines 53–54 with:

```bash
# Try the scanner: one package name, then a whole lockfile
cargo run --bin depradar -- name expresss
cargo run --bin depradar -- scan frontend/package-lock.json
```

- `expresss` (three `s`) is intentional. It is the typo of `express` used
  in tests and docs throughout the repo.
- Use a real lockfile path rather than a `<lockfile>` placeholder, so the
  command can be pasted as-is. `frontend/package-lock.json` is one of the
  lockfiles CI's `self-scan` job already scans.
- `scan` queries the OSV API, so it needs network access. Add `--offline`
  for typosquat checks only. It exits `1` when it finds anything, which is
  expected rather than an error (see `EXAMPLES` in `src/bin/depradar.rs`).

Also on line 97, the `typosquat` label row says "…or the `scan` binary."
It does not contain `--bin scan`, but it should say "the `depradar` CLI"
so the doc stays consistent.

### docs/issue-24-ecosystem-scoped-scoring.md

The `name` subcommand has **no** `--ecosystem` flag. In `depradar.rs` only
`scan` takes `--ecosystem`. So `cargo run --bin scan -- expresss --ecosystem
npm` has no one-to-one replacement. Replace the block with:

```
cargo run --bin depradar -- name expresss
cargo run --bin depradar -- scan frontend/package-lock.json --ecosystem npm
```

and drop the `requests --ecosystem pypi` line. Do not document a flag that
doesn't exist.

## Verification

```bash
grep -rn -- "--bin scan" --include=*.md .   # should print nothing (or only accepted history, see below)
cargo run --bin depradar -- name expresss   # flags expresss -> express
cargo run --bin depradar -- scan frontend/package-lock.json --offline
cargo run --bin depradar -- --help          # confirms subcommand and flag names
```

Note in the PR description that each command was run.

## Open Questions

- **History entries.** The acceptance criterion says "no docs mention
  `--bin scan`", and a literal grep includes `PROGRESS.md:371` and
  `README.md:304`. Those lines describe what PR #11 shipped at the time.
  Recommended: reword them to past tense ("added the `scan` CLI, since
  replaced by `depradar name`") so the grep is clean and the record stays
  accurate. Confirm with the maintainer in the issue thread.
- **Removing the binary.** Deleting `src/bin/scan.rs` is out of scope for
  this issue and belongs in its own issue.

## Implementation Checklist

- [ ] CONTRIBUTING.md lines 53–54 replaced; line 97 wording updated
- [ ] `docs/issue-24-ecosystem-scoped-scoring.md` examples replaced without `name --ecosystem`
- [ ] README/PROGRESS history lines handled as agreed in the thread
- [ ] Every command in the diff run locally
- [ ] PROGRESS.md entry with `Closes: #286` (the literal number, not `#<this issue>`)
- [ ] PR description contains `Closes #286`

## Notes

- #283 and #284 also edit CONTRIBUTING.md's setup block (lines 34–62).
  Whichever PR merges second will need a rebase.
