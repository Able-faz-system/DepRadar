# Issue #284 — `scripts/check.sh`: Run CI's Checks Locally

## Problem

CI runs six jobs in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml),
and the matching commands are spread across CONTRIBUTING.md. Contributors
push without running all of them and find failures in CI.

The two have also drifted apart. CONTRIBUTING says `npm run lint`, which is
`oxlint`, but CI runs `npx oxlint --deny-warnings`. Warnings pass locally
and fail in CI.

## Goal

One command that runs what `ci.yml` runs, in the same order and with the
same flags, and stops at the first failure.

## Affected Files

| File | Change |
|---|---|
| `scripts/check.sh` | New, committed with mode `100755` |
| `CONTRIBUTING.md` | Link the script from Development Setup and Pull Requests |
| `PROGRESS.md` | Add entry |

## What It Must Mirror

Taken from `ci.yml` as of this writing:

| CI job | Commands |
|---|---|
| `lint` | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `test` | `cargo test --workspace --locked`; `cargo test -p deprader --features async --locked` |
| `contract-wasm` | `cargo build -p deprader-router --target wasm32v1-none --release --locked` |
| `self-scan` | `cargo build --locked --bin depradar`, then `./target/debug/depradar --fail-on critical scan <f>` for `Cargo.lock`, `client/package-lock.json`, `frontend/package-lock.json` |
| `client` | in `client/`: `npm ci`; `npm run typecheck` |
| `frontend` | in `frontend/`: `npm ci`; `npx oxlint --deny-warnings`; `npm run typecheck`; `npm test`; `npm run build` |

Not included:

- `audit.yml` (`cargo audit`, `cargo deny`, `npm audit`) runs weekly and
  only on PRs that touch manifests. It is not part of every PR's checks, and
  it needs extra tools installed.
- `crates/typosquat-wasm` (`wasm32-unknown-unknown`) appears in
  CONTRIBUTING but no CI job builds it. The issue's "wasm build" means the
  contract build in `contract-wasm`.

## Sketch

```bash
#!/usr/bin/env bash
#
# Run the same checks as .github/workflows/ci.yml, stopping at the first
# failure. Keep this file in sync with ci.yml.
#
# Usage: scripts/check.sh
#
# Requires: stable Rust with rustfmt + clippy, the wasm32v1-none target,
# Node.js 22+, and network access (cargo/npm downloads, OSV lookups in the
# self-scan step).

set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

if ! rustup target list --installed | grep -qx wasm32v1-none; then
  echo "missing target: run 'rustup target add wasm32v1-none'" >&2
  exit 2
fi

step "lint: fmt"
cargo fmt --all -- --check
step "lint: clippy"
cargo clippy --workspace --all-targets --locked -- -D warnings

step "test"
cargo test --workspace --locked
step "test (async feature)"
cargo test -p deprader --features async --locked

step "contract-wasm"
cargo build -p deprader-router --target wasm32v1-none --release --locked

step "self-scan"
cargo build --locked --bin depradar
for lockfile in Cargo.lock client/package-lock.json frontend/package-lock.json; do
  ./target/debug/depradar --fail-on critical scan "$lockfile"
done

step "client"
(cd client && npm ci && npm run typecheck)

step "frontend"
(cd frontend && npm ci && npx oxlint --deny-warnings && npm run typecheck && npm test && npm run build)

printf '\n\033[1;32mAll CI checks passed.\033[0m\n'
```

Design points:

- **`set -euo pipefail`** stops the script at the first failing command, as
  the issue requires. The `&&` chains inside the subshells fail the same way.
- **`cd` to the repo root**, so the script works from any directory.
- **`npm ci`** rather than `npm install`, as CI uses. It deletes
  `node_modules`, which is slower but reproduces CI exactly.
- **Exit code `2`** for a missing prerequisite. That is different from a
  check failing, and it matches the CLI's convention of 2 for usage errors.
- **Executable bit.** `scripts/deploy.sh` is tracked as `100644`. Make sure
  this script is `100755`:
  `git update-index --chmod=+x scripts/check.sh`.

Optional additions, only if the maintainers agree in the thread: a
`--rust-only` / `--frontend-only` switch, or `--fix` to run `cargo fmt
--all` first. Neither is required by the issue.

## CONTRIBUTING.md

In **Development Setup**, above the command block:

```markdown
To run everything CI runs, in the same order, use:

    scripts/check.sh
```

In **Pull Requests**, change "Keep CI green: …" to mention running
`scripts/check.sh` before pushing. Also change the frontend line 61 to
`npx oxlint --deny-warnings` (or point it at the script), so CONTRIBUTING
no longer disagrees with CI.

## Verification

- On a clean checkout of `main` (green in CI), `scripts/check.sh; echo $?`
  prints `0`. Run it from both the repo root and `scripts/`.
- Break something on purpose, such as an unformatted line. The script must
  stop at `lint: fmt` with a non-zero exit and run nothing after it.
- The PR's own CI run is green.

## Implementation Checklist

- [ ] `scripts/check.sh` covers every command in the table, with CI's flags
- [ ] Tracked as `100755`
- [ ] Exits 0 on a clean green checkout; stops at the first failure otherwise
- [ ] CONTRIBUTING links the script; frontend lint command matches CI
- [ ] PROGRESS.md entry with `Closes: #284` (the literal number)
- [ ] PR description contains `Closes #284`

## Notes

- If a job is added to `ci.yml` later, this script needs the same change.
  The header comment says so, and adding a matching comment at the top of
  `ci.yml` makes it harder to miss.
- #283 adds a `ci-success` job. It runs no commands, so this script doesn't
  need a step for it.
