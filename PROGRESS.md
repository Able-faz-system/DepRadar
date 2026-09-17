# Progress Log

This file is the running record of concrete work landed in this repo: what
was built, who built it, and which issue (if any) it closes. It exists so
that "what's actually shipped" is answerable by reading one file, rather
than reconstructing it from git history or a scattered issue tracker.

Every issue's Definition of Done requires an entry here before the issue
is closed — see [CONTRIBUTING.md](CONTRIBUTING.md).

## Entries

### deprader_router contract — create_bounty / submit_claim / release

- **What:** Initial Soroban smart contract at `contracts/deprader_router/`.
  Implements `initialize(admin)`, `create_bounty(creator, bounty_id,
  issue_ref, amount, token)` (locks funds against a bounty ID),
  `submit_claim(bounty_id, claimant, proof_ref)` (records a claim, no fund
  movement), and `release(admin, bounty_id)` (admin-gated, one-time payout
  of locked funds to the claimant). Getters: `get_bounty`, `get_claim`.
- **By:** ALLEN-AYODEJI
- **Closes:** N/A (predates the issue tracker)
- **Status:** Deployed and verified on Stellar testnet.
  - Contract ID: `CBFLUY6NB3JOQDEUOF2JG5TL3UD7GCQ7APUJ6EHNNSUJLUELHSGIG675`
  - A full `create_bounty` → `submit_claim` → `release` cycle was run
    against the live testnet contract using the native XLM Stellar Asset
    Contract as the test token, and confirmed via `get_bounty` / `get_claim`
    reads and on-chain transfer events.

### Typosquat — popular-package reference dataset and loader

- **What:** Curated reference lists of the npm and PyPI top-1000 packages
  (`data/popular-packages/npm-top1000.json` and
  `data/popular-packages/pypi-top1000.json`) and the dataset loader module
  (`src/typosquat/dataset.rs`) that reads them into the typosquat pipeline.
  These lists are the ground truth a submitted package name is compared
  against to detect squatting candidates.
- **By:** ALLEN-AYODEJI
- **Closes:** [PR #1](https://github.com/ALLEN-AYODEJI/DepRadar/pull/1)
- **Status:** Merged.

### Typosquat — Levenshtein edit-distance scorer and `nearest_match`

- **What:** `src/typosquat/distance.rs` — a Levenshtein edit-distance
  implementation and a `nearest_match(name, dataset)` function that returns
  the closest package name from the popular-package reference lists along
  with its edit distance. This is the first scoring signal in the typosquat
  detection pipeline; subsequent signals (homoglyph, keyboard-adjacency)
  will be merged into a combined scorer on top of this foundation.
- **By:** ALLEN-AYODEJI
- **Closes:** [PR #2](https://github.com/ALLEN-AYODEJI/DepRadar/pull/2)
- **Status:** Merged.
