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

### typosquat scanner — popular-package reference data + loader

- **What:** Groundwork for the typosquat detection scanner. Adds
  `data/popular-packages/npm-top1000.json` and
  `data/popular-packages/pypi-top1000.json` — each a JSON array of ~1000
  widely-used package names for that registry (a hand-curated representative
  snapshot, not a live-fetched ranking). Introduces the crate's off-chain
  Rust side: a root `deprader` library crate with
  `src/typosquat/dataset.rs`, exposing `load_popular_packages()`, which
  embeds both lists at compile time and returns them as one combined
  `Vec<String>` (npm first, then PyPI). No scoring logic yet — just the data
  and the loader. Covered by a unit test asserting the combined list is
  non-empty and contains known packages (`react`, `requests`).
- **By:** ALLEN-AYODEJI
- **Closes:** N/A (prep for the typosquat scanner module)
- **Status:** Merged. `cargo test --workspace --locked` passes.

### typosquat scanner — edit-distance primitives

- **What:** Adds `src/typosquat/distance.rs`. `levenshtein_distance(a, b)`
  computes standard Levenshtein edit distance (insert / delete / substitute,
  no transposition op) over Unicode scalar values with the two-row DP table,
  no external crate. `nearest_match(candidate, dataset)` returns the closest
  name in a reference slice and its distance (earliest entry wins ties;
  empty slice yields `(String::new(), usize::MAX)`). Wired into
  `src/typosquat/mod.rs`. Still no scoring/thresholding — callers decide
  what distance counts as suspicious.
- **Tests:** identical strings → 0; empty-operand cases; one-character-off
  typosquats (`expres`/`expresss` vs `express`, a single substitution) → 1;
  transposition → 2; `nearest_match` against the loaded popular-package
  dataset resolves deliberately misspelled `expresss` → `express` and
  `requezts` → `requests`, both at distance 1; exact hit → distance 0;
  empty-dataset sentinel.
- **By:** ALLEN-AYODEJI
- **Closes:** N/A (prep for the typosquat scanner module)
- **Status:** Merged. `cargo test --workspace --locked` passes.

### typosquat scanner — homoglyph (lookalike-character) signal

- **What:** Adds `src/typosquat/homoglyph.rs`, a second detection signal that
  sits alongside the edit-distance one and shares no code with it.
  `normalize_homoglyphs(candidate)` rewrites a name by mapping known Unicode
  lookalike characters back to their Latin equivalents — a hand-picked,
  deliberately conservative subset of the Unicode `confusables.txt` (UTR #39):
  Cyrillic, Greek and Armenian letters plus a couple of Latin-extended/IPA
  forms, and the entire full-width ASCII block (`U+FF01..=U+FF5E`) handled as a
  contiguous range. `homoglyph_impersonation(candidate, dataset)` returns
  `Some(name)` only when a substitution actually happened *and* the normalised
  form exactly matches an entry in `dataset` (in practice the combined list
  from `load_popular_packages()`); a correctly-spelled popular package
  normalises to itself and is never flagged. Wired into `src/typosquat/mod.rs`.
  Still no scoring/thresholding — this is a boolean signal for the
  (not-yet-written) scanner to weigh.
- **Tests:** `normalize_homoglyphs` is identity for plain ASCII (incl. a
  scoped `@angular/core` name and the empty string); Cyrillic `а` (U+0430) and
  `о` (U+043E) in `react`/`lodash` → Latin; full-width `ｅｘｐｒｅｓｓ` → `express`;
  Greek omicron (U+03BF) in `commander` → `o`. `homoglyph_impersonation`
  against the loaded popular-package dataset flags a Cyrillic-`а` `react`
  (npm), a full-width `requests` (PyPI) and a Greek-omicron `lodash` (npm);
  returns `None` for a correctly-spelled `react` and for a homoglyph name
  (`reаctxyzzy`) whose normalised form is not a popular package.
- **By:** ALLEN-AYODEJI
- **Closes:** N/A (prep for the typosquat scanner module)
- **Status:** Merged. `cargo test --workspace --locked` passes.

### Testnet read client — get_bounty / get_claim by ID

- **What:** Adds `client/`, a minimal read-only Node/TS client for
  `deprader_router` on Stellar testnet (contract ID
  `CBFLUY6NB3JOQDEUOF2JG5TL3UD7GCQ7APUJ6EHNNSUJLUELHSGIG675`). Uses
  `@stellar/stellar-sdk`'s `rpc.Server.queryContract` to simulate
  `get_bounty(bounty_id)` and `get_claim(bounty_id)` calls against
  `https://soroban-testnet.stellar.org` and print the decoded result — no
  transactions are submitted and no funded account is required, since both
  contract methods are pure reads. CLI: `npm start -- --bounty-id <id>
  --claim-id <id>` (both default to `1`). An ID with no matching bounty or
  claim prints a plain "not found" line rather than erroring, matching the
  contract's `Option<Bounty>` / `Option<Claim>` return type. Kept as a
  standalone `client/` package (its own `package.json`/`tsconfig.json`,
  outside the Cargo workspace) since it's plain TypeScript, not a Soroban
  contract.
- **Verified:** Run against the live contract with the default IDs, which
  resolve to the real `create_bounty` → `submit_claim` → `release` cycle
  recorded above — `get_bounty(1)` returned the stored bounty (amount
  `50000000`, `issue_ref: "org/repo#123"`, `released: true`) and
  `get_claim(1)` returned the stored claim (`proof_ref:
  "org/repo#123-pr-456"`). Also verified an unknown ID (`999`) prints the
  not-found path for both calls instead of throwing.
- **By:** ALLEN-AYODEJI
- **Closes:** #6
- **Status:** Merged. No UI yet, per the issue scope — read-only CLI output
  only.

### CVE matching — OSV API client

- **What:** Adds `src/cve/osv_client.rs`, the first piece of the CVE
  matching scanner class. `query_osv(package_name, ecosystem, version)`
  queries the [OSV.dev](https://osv.dev) API (`POST /v1/query`) for a package
  pinned to an exact version and parses the response into typed structs
  (`OsvQueryResponse` → `Vec<OsvVulnerability>`, with nested `OsvAffected` /
  `OsvRange` / `OsvEvent` / `OsvSeverity` / `OsvReference`) mirroring the
  relevant parts of the [OSV schema](https://ossf.github.io/osv-schema/). The
  `ecosystem` argument is a typed `Ecosystem` enum (`Npm`, `PyPI`,
  `CratesIo`) rather than a free string, mapped to the exact ecosystem names
  OSV expects. No filtering, deduplication, or scoring — an empty `vulns`
  list just means OSV has no known match for that exact version; turning raw
  matches into scored findings is later work. Uses `reqwest` (blocking
  client) for HTTP and `serde`/`serde_json` for (de)serialization — new
  dependencies for the crate, added to `Cargo.toml`.
- **Tests:** `query_osv` against the real OSV API for `lodash@4.17.4` (npm),
  a known-vulnerable pair predating the fix for GHSA-29mw-wpgm-hmr9 /
  CVE-2020-28500 (ReDoS in `trim`/`toNumber`/`trimEnd`, fixed in 4.17.21) —
  asserts the response is non-empty and includes that CVE alias. The test
  skips (rather than fails) if the request itself can't complete, since that
  reflects the runner's network access rather than a bug in the client. Also
  a plain unit test that `Ecosystem::as_osv_str()` matches OSV's documented
  ecosystem names for `npm`/`PyPI`/`crates.io`.
- **By:** ALLEN-AYODEJI
- **Closes:** N/A (prep for the CVE matching scanner module)
- **Status:** Merged. `cargo test --workspace --locked` passes, including a
  live network call to `api.osv.dev` confirming the known-vulnerable test
  case.

### typosquat scanner — combined scoring API + CLI

- **What:** Adds `src/typosquat/scanner.rs`, the single entry point that ties
  the two independent typosquat signals together:
  `score_package(name: &str) -> ScanResult` loads
  [`load_popular_packages()`](src/typosquat/dataset.rs) internally, runs
  `nearest_match` ([`distance.rs`](src/typosquat/distance.rs)) and
  `homoglyph_impersonation` ([`homoglyph.rs`](src/typosquat/homoglyph.rs))
  against it, and returns a `ScanResult` recording which signal(s) fired and
  why. A `distance_match` only counts when the nearest name is within
  `DISTANCE_THRESHOLD` (2) edits and isn't an exact match (distance 0 is the
  real package, not a typosquat); `homoglyph_match` is whatever
  `homoglyph_impersonation` reports. `ScanResult::is_likely_typosquat()` is
  true if either fired, and either, both, or neither can fire independently
  since the two signals still share no logic. `ScanResult` implements
  `Display` for human-readable output (used by the CLI below). Also adds
  `src/bin/scan.rs`, a small binary (`cargo run --bin scan -- <package-name>`)
  that scores its one argument and prints the result — the manual-testing
  surface the issue asked for. Wired into `src/typosquat/mod.rs`.
- **Tests:** exact popular-package name (`react`) and a long unrelated
  string are both unflagged; a pure edit-distance typo (`expresss` →
  `express`, distance 1) flags only `distance_match`; a full-width `requests`
  flags only `homoglyph_match` (its raw-codepoint edit distance to any
  dataset entry is 8, well past the threshold); a Cyrillic-`а` `react`
  flags *both* (it's simultaneously one substitution away from `react` and a
  homoglyph impersonation of it) — the concrete case the issue asked for.
  Two more tests check `Display` output for the both-signals and no-signal
  cases.
- **By:** ALLEN-AYODEJI
- **Closes:** #4
- **Status:** Pushed on `typosquat-combined-scoring`, pending review/merge.
  Depended on #2 (dataset/distance) and the homoglyph signal (#7), both
  already merged.
