# Day 10 — Evil-merge detector

**Issue:** [#298](https://github.com/DepRadarOrg/DepRadar/issues/298) ·
**Depends on:** Day 9 (merged first) ·
**Relevant files:** `src/`

## Goal

An *evil merge* is a merge commit that introduces changes that came from
neither side of the merge. Reviewers look at each branch's commits, and
`git log -p` hides merge diffs by default, so content slipped into the
merge commit itself often goes unseen.

**Acceptance test:** a merge commit that slips in an extra file is flagged.

## Detection

Run this for each merge commit `M` with parents `P1 … Pn` (n ≥ 2). Reuse
the Day 9 commit walker.

### Tier 1: paths that come from no parent (high confidence)

Compare `M`'s tree with each parent's tree, path by path.

- **Added from nowhere:** the path exists in `M` but in **none** of the
  parents. This is the acceptance case.
- **Deleted from nowhere:** the path exists in **every** parent but not
  in `M`.

Neither case can come from a normal merge.

### Tier 2: content that matches no parent (needs care)

A path whose blob in `M` differs from the blob at that path in **every**
parent. This is what the issue describes as "matches neither parent".

**Don't flag on that alone.** A clean three-way merge also produces this
result legitimately. If `P1` edits line 1 of a file and `P2` edits line 10,
the merged blob matches neither parent. Flagging every such file would
bury the real findings.

Instead, compute what git would have produced automatically and compare
`M` against *that*:

```text
git merge-tree --write-tree P1 P2      # requires git >= 2.38
```

- The auto-merge result is clean and its blob equals `M`'s blob: this is a
  normal merge. **Don't flag.**
- The auto-merge result is clean but differs from `M`: **flag.** Someone
  edited the file during the merge.
- The auto-merge had a conflict on that path: the author had to resolve
  it by hand. Report at **low** severity as "manual conflict resolution;
  review recommended", or skip it. Pick one and document the choice.

For octopus merges (n > 2), apply Tier 1 as written. For Tier 2, report at
low severity without the auto-merge comparison, because `merge-tree` only
takes two branches.

Ignore mode-only changes, or report them as a separate kind. Submodule
(gitlink) entries count as normal paths.

## Suggested shape

```rust
pub enum EvilMergeKind {
    AddedFromNowhere { path: String },
    DeletedFromNowhere { path: String },
    ContentFromNowhere { path: String },      // tier 2, clean auto-merge disagrees
    ManualConflictResolution { path: String } // tier 2, low severity
}
```

Each finding carries the merge SHA and its parent SHAs.

## Fixtures (built at test time)

**Positive (must flag):**
- Branches `main` and `feature`, each with a normal commit. Merge with
  `--no-commit`, add `backdoor.js` (defanged content:
  `// defanged fixture`), then commit. This is the acceptance case and
  should produce `AddedFromNowhere`.
- The same setup, but before committing the merge, edit a file that only
  one side touched. Should produce `ContentFromNowhere`.
- A merge that deletes a file present in both parents. Should produce
  `DeletedFromNowhere`.

**Negative (must not flag):**
- A plain `git merge --no-ff` with no extra edits.
- **A clean three-way merge of the same file.** `main` edits line 1,
  `feature` edits line 10, and git merges them automatically. This is the
  test that proves Tier 2 doesn't false-positive.
- A fast-forward (no merge commit at all).
- A Day 9 style single-parent commit. It isn't a merge, so this detector
  should skip it.

## Implementation notes

- If the earlier days access git through the CLI, `merge-tree
  --write-tree` is one more `Command` call. If they use a library
  (`gix`/`git2`), check whether it offers an equivalent in-memory merge.
  If it doesn't, the CLI fallback is fine. Note in the README that
  git ≥ 2.38 is required.
- Diff trees by object ID. Don't read blob contents except where Tier 2
  needs them.
- Don't let a single huge merge commit dominate runtime. Skip binary
  blobs in Tier 2.
