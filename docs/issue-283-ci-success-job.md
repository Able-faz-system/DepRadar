# Issue #283 — Add a `ci-success` Job to Use as the Required Status Check

## Problem

PRs have been merged while CI was red. Branch protection can require status
checks, but requiring each of the six jobs in
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) individually is
fragile: whenever a job is added or renamed, the protection rule has to be
updated by hand, and a forgotten job silently stops being required.

## Goal

A single job, `ci-success`, that:

- depends on every other job in `ci.yml`;
- always runs, even when a dependency failed (otherwise GitHub marks it
  *skipped*, and a skipped required check does **not** block merging);
- fails unless every dependency succeeded.

Branch protection then requires just `ci-success`.

## Affected Files

| File | Change |
|---|---|
| `.github/workflows/ci.yml` | Add the `ci-success` job |
| `CONTRIBUTING.md` | State that PRs merge only when `ci-success` is green |
| `PROGRESS.md` | Add entry |

## Design

The current jobs are `lint`, `test`, `contract-wasm`, `self-scan`,
`client` and `frontend`. Append:

```yaml
  # Aggregate check for branch protection. Every job above must be listed
  # in `needs:` — add new jobs here too.
  ci-success:
    name: ci-success
    if: always()
    needs: [lint, test, contract-wasm, self-scan, client, frontend]
    runs-on: ubuntu-latest
    steps:
      - name: Check that every job succeeded
        env:
          RESULTS: ${{ join(needs.*.result, ' ') }}
        run: |
          echo "Job results: $RESULTS"
          for result in $RESULTS; do
            if [ "$result" != "success" ]; then
              echo "::error::A required CI job finished with '$result'"
              exit 1
            fi
          done
```

Why it is built this way:

- **`if: always()`** makes the job run after failed or cancelled
  dependencies. Without it the job is skipped, and branch protection treats
  a skipped check as passing.
- **The check is "anything other than `success`"**, not just "contains
  `failure`". A cancelled or skipped dependency also fails the gate, so a
  timed-out job can't let a PR through.
- **The results go through `env:`** rather than being interpolated into
  the script. This follows GitHub's guidance on keeping expressions out of
  `run:`, even though the values here are fixed strings.
- **`name: ci-success`.** The status check is named after the job's
  `name:`, so this is the exact string to select in branch protection.
- **No checkout or toolchain steps.** The job does nothing else and
  finishes in seconds.

`audit.yml` stays out of `needs:`. It is a separate workflow, and it only
runs on PRs that touch manifests (`paths:` filter), so requiring it would
block every other PR.

## CONTRIBUTING.md

In **Pull Requests**, replace the "Keep CI green" bullet with something
like:

```markdown
- PRs merge only when the **`ci-success`** check is green. It passes only
  if every job in `ci.yml` (fmt, clippy, tests, the wasm build, self-scan,
  client typecheck and the frontend checks) succeeded.
```

Make the same change to the matching line of the reviewer checklist.

## Verification

1. Open the PR. `ci-success` appears and passes once the other six pass.
2. Push a temporary commit that breaks one job, for example an unformatted
   line for `lint`. `ci-success` must run and **fail**, not be skipped.
   Revert the commit. Link both runs in the PR description as evidence for
   "fails when any other job fails and passes when all pass".
3. Final run on the PR is green.

## Maintainer Follow-up (outside the PR)

A contributor can't make the check required. After merge, a repo admin
adds `ci-success` as a required status check on `main` (Settings →
Branches / Rulesets) and can remove the per-job requirements. Mention this
in the PR description so it isn't missed.

## Implementation Checklist

- [ ] `ci-success` job added with `if: always()` and all six jobs in `needs:`
- [ ] Fails on any non-`success` result
- [ ] Shown failing (deliberate break) and passing on the PR, with run links
- [ ] CONTRIBUTING updated in Pull Requests and the reviewer checklist
- [ ] PROGRESS.md entry with `Closes: #283` (the literal number)
- [ ] PR description contains `Closes #283` and the branch-protection follow-up

## Notes

- The `needs:` list must be kept up to date by hand. The comment above the
  job is there so whoever adds the next job sees it.
- #284 (`scripts/check.sh`) mirrors the same jobs locally. `ci-success`
  runs no commands, so the script doesn't change.
