# Day 9 — Forged-merge detector

**Issue:** [#297](https://github.com/DepRadarOrg/DepRadar/issues/297) ·
**Depends on:** Day 8 (merged first) ·
**Relevant files:** `src/`

## Goal

Flag commits that *claim* to be GitHub PR merges but aren't. GitHub's
"Create a merge commit" button always produces a commit with **two
parents**. A commit with **one parent** whose subject reads
`Merge pull request #N from …` was written by hand to borrow the trust
reviewers give to merged PRs.

**Acceptance test:** reproduce the Trellis finding on a synthetic repo.
That is a single-parent commit with a GitHub-style merge title, slipped
into an otherwise normal history.

## Detection

### Local check (required)

Walk the commits being scanned. Reuse the commit walker from the earlier
days if one exists, because Day 10 needs it too. Flag a commit when both
of these hold:

1. It has exactly **one** parent.
2. Its **subject line** (the first line of the message only) matches:

   ```text
   ^Merge pull request #(\d+) from \S+
   ```

Capture `N` and the `from` ref for the finding. Don't match against the
message body, because a normal commit can quote a merge title in its body.

A root commit (zero parents) with this title is also forged. Flag it as
well, since it's cheap to cover.

### GitHub API confirmation (optional)

When the scan knows the repo's `owner/repo` and the network is available,
call:

```text
GET https://api.github.com/repos/{owner}/{repo}/pulls/{N}
```

| Response | Meaning |
|---|---|
| `merged: true` and `merge_commit_sha == <commit sha>` | GitHub vouches for it. Downgrade or drop the finding. |
| `merged: true` and a different `merge_commit_sha` | **Confirmed forgery.** The real merge is another commit. |
| `merged: false`, or 404 | **Confirmed forgery.** That PR was never merged, or doesn't exist. |
| Network error or rate limit | Keep the local finding, marked "unconfirmed". |

- Use the existing `reqwest` blocking client. Don't add a new HTTP dependency.
- Read an optional `GITHUB_TOKEN` from the environment. Unauthenticated
  requests are limited to 60 per hour.
- Network tests must skip when offline, as `src/cve/` does.

## Fixtures (built at test time)

**Positive (must flag):**
- History `A → B → F → C`, where `F` has one parent and the subject
  `Merge pull request #42 from attacker/feature`. This is the Trellis
  reproduction.
- A root commit with a `Merge pull request #1 from x/y` subject.

**Negative (must not flag):**
- A real two-parent merge (`git merge --no-ff`) with the subject
  `Merge pull request #7 from dev/feature`.
- A single-parent commit titled `Merge branch 'main' into feature`. This
  is a different pattern and out of scope.
- A squash-merge style commit titled `Add parser (#12)`.
- A single-parent commit whose *body* (not its subject) contains
  `Merge pull request #3 from a/b`.

## Known false positive

`git cherry-pick -m 1 <merge>` and some history rewrites can produce a
legitimate single-parent commit that keeps the original merge title. The
local check can't tell this apart from a forgery. Report it at medium
severity when unconfirmed, and let the API check settle it. Document this
in the finding text.

## Notes for Day 10

Day 10 will walk merge commits with the same walker. Keep the parent
lookup and the commit iteration in shared code, not inside this detector.
