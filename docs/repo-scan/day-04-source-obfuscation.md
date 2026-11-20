# Day 4 — Obfuscation detector over repository source

**Issue:** [#292](https://github.com/DepRadarOrg/DepRadar/issues/292) ·
**Depends on:** Day 3 (merged first) ·
**Relevant file:** `src/behavioral/obfuscation.rs`

## Goal

Run the existing `behavioral::obfuscation` scorer over the repository's own
JavaScript and TypeScript, so packed or encoded loaders committed to the
repo get flagged. The current thresholds were tuned for **published package
tarballs**. Hand-written source in a repository looks different, so it needs
its own profile.

**Acceptance tests:**
- Packed or encoded loaders are flagged.
- Committed bundles that `depradar.toml` lists are ignored.

## What exists today

[`obfuscation.rs`](../../src/behavioral/obfuscation.rs) scores a file
on these signals:

| Signal | Weight |
|---|---|
| Base64 run of 200+ chars in a string | 3 |
| Hex run of 200+ chars in a string | 4 |
| `eval(` / `new Function(` / `atob(` in code | 2 / 3 / 2 |
| High-entropy (≥ 4.8) string of 128+ chars | 2 each, max 4 |
| Line over 2,000 chars | 1 |

It reports a file at a score of 4 or more (`OBFUSCATION_THRESHOLD`) and
rates 8 or more as high. `scan_tree` only looks at `.js`, `.mjs` and `.cjs`,
skips files over 5 MiB, and walks with `collect_files`, which ignores
`.gitignore` and `depradar.toml`.

## Tasks

### 1. A threshold profile

Move the thresholds and weights into a struct. Keep today's values as the
tarball profile.

```rust
pub struct ObfuscationProfile {
    pub threshold: u32,
    pub high_threshold: u32,
    pub long_line: usize,
    pub blob_min_len: usize,
    pub entropy_min_len: usize,
    pub high_entropy: f64,
    pub weights: Weights,
}

impl ObfuscationProfile {
    pub const TARBALL: Self = /* today's constants, unchanged */;
    pub const SOURCE: Self = /* tuned for repository source */;
}

pub fn score_with(source: &str, profile: &ObfuscationProfile) -> ObfuscationScore;
```

**Every existing public function keeps its current behavior** by calling
`score_with(…, &ObfuscationProfile::TARBALL)`. The existing tests must
pass unchanged. That proves the refactor didn't move the package-scan
results. Keep the public `const`s as aliases for the `TARBALL` values, so
nothing that imports them breaks.

### 2. Tuning the `SOURCE` profile

Code that people write and review has different norms from published
output:

- **A very long line is more suspicious.** Nobody writes a 2,000-character
  line by hand. Suggested change: `long_line: 1_000` with weight 2. A
  long line alone still stays under the threshold.
- **`eval` and `new Function` are rarer** in application code than in
  bundled dependencies. Their weights can stay the same, but don't lower
  the threshold just because of that. Day 3 already catches the padding
  trick, and a lower threshold mostly adds noise.
- **Base64 has legitimate uses in source.** Examples are `data:image/…`
  and `data:font/…` URIs, inline WASM, and test fixtures. Don't exempt
  them outright, because attackers use the same disguise. Consider
  counting a blob that sits inside a `data:image/` or `data:font/` URI
  at weight 1 instead of 3.
- **High-entropy strings have legitimate uses too.** SRI hashes
  (`sha384-…`), PEM public keys and sample JWTs in tests all score high.
  Cap the entropy contribution as it is today. Don't raise it.

These are starting points. **Tune them with data:** run the profile over
this repo and over a few popular JS/TS repositories, and record the hit
counts before and after in the PR description. If a value needs to change,
the PR is where to argue it.

### 3. File selection

Scan `.js`, `.mjs`, `.cjs`, `.ts`, `.mts` and `.cts`. Also include `.jsx`
and `.tsx`: they run the same way, and leaving them out would be an obvious
gap. Get the file list from **the Day 1 walker**, not `collect_files`, so
`.gitignore` and `exclude` are respected. Keep the 5 MiB limit and the
"skip non-UTF-8 and unreadable files" behavior.

### 4. Skipping committed bundles

The issue says to skip `dist/` bundles that are listed in config. Two
design points:

- **Don't skip `dist/` by default.** `dist/setup.js` is a PolinRider
  indicator (Day 7). A default skip would blind the scanner to it. Skip
  only what the user lists.
- **Use a separate key, not `exclude`.** `exclude` removes a file from
  *every* detector. A committed bundle should still get Day 2's IOC check
  and Day 3's padding check. It should only stop getting obfuscation
  scores, which minified code always triggers. Suggested:

  ```toml
  [repo-scan]
  bundles = ["dist/**", "public/vendor/*.min.js"]
  ```

  Parse it in [`src/config.rs`](../../src/config.rs) with the same glob
  semantics as `exclude`. Add a parse test and a test for unknown keys,
  matching the existing ones.

`*.min.js` files that aren't listed are still scanned. Their score is
usually only the long-line point, so they aren't flagged anyway.

## Fixtures

All of these must be defanged. The decoded content of every blob is
`console.log("defanged fixture")`, padded or repeated to the required
length. No real loader.

**Positive (must flag):**
- `src/loader.js`: a 300-char base64 string passed to `atob`, with the
  result handed to `new Function(…)()`. Expected score ≥ 8, high.
- `scripts/init.ts`: a hex blob decoded with `Buffer.from(…, 'hex')` and
  passed to `eval`.
- `src/packed.mjs`: one 1,200-char line containing an `eval` call
  (`SOURCE` long line + eval). This is flagged under `SOURCE` but **not**
  under `TARBALL`. Assert both, to prove the profiles differ.

**Negative (must not flag):**
- `dist/bundle.js`: a minified, high-scoring file listed under `bundles` in
  a test `depradar.toml`. It must be ignored by this detector, but Day 3's
  detector must still see it.
- `dist/other.js`: the same file *not* listed under `bundles`. This one is
  scored, so it is a positive. It shows the skip is config-driven.
- `src/icons.ts` with several `data:image/svg+xml;base64,…` URIs.
- `src/integrity.ts` with SRI hashes and a PEM public key.
- An ordinary TypeScript module and a React `.tsx` component.

## Known traps

- **Self-scan.** The positive fixtures will trip this detector on
  DepRadar's own repo. Use the same approach Day 3 picked (exclude
  `tests/fixtures/repo-scan/**`, or build the fixtures in the test).
- **`frontend/` and `client/` in this repo.** Running on DepRadar itself
  will scan them. Check that they produce nothing. If they do, tune the
  profile; don't add an exclude to make the result disappear.

## Out of scope

- Deobfuscating or decoding payloads.
- A JavaScript parser. The existing string and comment tokenizer is enough.
