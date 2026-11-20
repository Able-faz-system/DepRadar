# Issue #285 — Fill in the `#<this issue>` Placeholders in PROGRESS.md

## Problem

The issue templates end with "PR description includes `Closes #<this
issue>`". Some contributors copied that text literally into
[PROGRESS.md](../PROGRESS.md), so those entries don't link to any issue.

## Current State

`grep -n '#<this issue>' PROGRESS.md` finds **one entry**, with the
placeholder appearing twice:

| Line | Text |
|---|---|
| 86 | `### Docs — FAQ — #<this issue>` |
| 97 | `- **Closes:** #<this issue>` |

## Finding the Real Number

`git log -S'#<this issue>' -- PROGRESS.md` shows that the placeholder came
from a single commit:

```
63e7ae9 security: [docs] Add a FAQ (#160)
```

It was merged as:

```
36cc751 Merge pull request #260 from Osaodion/security/issue-160-add-a-faq
```

The branch name (`issue-160-…`) and the commit subject both point to
**issue #160**. PR #260 is the pull request, not the issue. Before writing
the number, check that the PR #260 description on GitHub says `Closes #160`
(or that issue #160 is titled "Add a FAQ").

## Change

```diff
-### Docs — FAQ — #<this issue>
+### Docs — FAQ — #160
 ...
-- **Closes:** #<this issue>
+- **Closes:** #160
```

Then add this issue's own entry. It needs the literal number `#285`. If it
contains `#<this issue>`, the PR re-introduces the bug it fixes.

## Verification

```bash
grep -n '#<this issue>' PROGRESS.md   # must print nothing
grep -n '#160' PROGRESS.md            # the FAQ entry heading and Closes line
```

## Open Questions

- **Author mismatch.** The FAQ entry says `**By:** CodePrincess360`, but
  commit `63e7ae9` was authored by `Dev_Osas` and merged from the
  `Osaodion` fork. This is out of scope for #285. Raise it in the issue
  thread rather than changing it silently.
- **Root cause.** The issue template wording is what produces these
  placeholders. Changing the templates in `.github/ISSUE_TEMPLATE/` to say
  "`Closes #N`, using the issue's number" would prevent this from
  happening again. That belongs in its own issue.

## Implementation Checklist

- [ ] PR #260 on GitHub confirmed to close #160
- [ ] Both placeholders on lines 86 and 97 replaced with `#160`
- [ ] `grep '#<this issue>' PROGRESS.md` returns nothing
- [ ] PROGRESS.md entry for #285 added, with the literal number
- [ ] PR description contains `Closes #285`
