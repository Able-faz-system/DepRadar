# Day 3 — Whitespace-padding detector

**Issue:** [#291](https://github.com/DepRadarOrg/DepRadar/issues/291) ·
**Depends on:** Day 2 (merged first) ·
**Relevant files:** `src/` (new module, e.g. `src/repo_scan/padding.rs`)

## Goal

Find code hidden far to the right of the visible text. PolinRider added
its payload to build configs such as `postcss.config.js`. It put the payload
on the same line as legitimate code, after hundreds of spaces, so it sat
past the edge of every editor and diff view. A reviewer saw a normal config.
Node ran both.

**Acceptance tests:**
- A defanged copy of the Trellis `postcss.config.js` payload is caught.
- None of the 20 most common real config files (listed below) produce a
  finding.

## What to detect

There are three checks. Each one produces its own finding kind, so a report
says *why* a line was flagged.

### 1. Code after a long run of whitespace

On any line, find a run of at least `PADDING_RUN` spaces or tabs that is
followed by a non-whitespace character. **This is the main signal.** Indented
code never has hundreds of blank columns *followed by* more code.

- Suggested starting value: `PADDING_RUN = 150`. Make it a `const` and
  tune it against the negative corpus. Trailing whitespace at the end of a
  line doesn't count. Only whitespace with code *after* it counts.
- Treat a tab as one character for the run length. Don't expand tabs to
  columns, so results don't depend on tab width.
- Handle `\r\n` line endings. Strip the `\r` before measuring.
- Report the **column** where the hidden code starts, plus a snippet of the
  hidden code truncated the same way as
  `behavioral::network::MAX_SNIPPET_CHARS` (80). Never put the whole payload
  in a report.

### 2. Code after the module export in `*.config.*` files

In a file whose name matches `*.config.{js,cjs,mjs,ts,cts,mts}`, find the
end of the top-level `export default …` or `module.exports = …`
statement. Flag any code that follows it **on the same line**.

The config value is usually a multi-line object, so you need to find where
that statement ends:

- Track `()`, `[]` and `{}` depth from the export keyword until depth
  returns to 0 and the statement ends (`;`, a newline at depth 0, or EOF).
- Skip brackets inside string literals, template literals (including
  `${…}`) and comments. `behavioral::obfuscation` already tracks string and
  comment state. Reuse that logic rather than writing a second tokenizer.
- Don't try to fully parse JavaScript. If the scanner loses track of
  depth, for example on unbalanced brackets, give up on that file
  quietly. Check 1 still runs.

Code on *later lines* after the export is also in scope, but it is often
legitimate (see the false-positive traps below). Flag it only when it is not
a function or class declaration, a comment, or a further
`module.exports.x =` assignment. Rate it lower than same-line trailing code.

### 3. Very long lines

A line longer than `LONG_LINE` characters, in a text source file that isn't
minified. Use `behavioral::obfuscation::LONG_LINE` (2,000) rather than
defining a second constant. This is the weakest signal, so report it as low
severity.

- Apply it only to source and config extensions (`.js`, `.cjs`, `.mjs`,
  `.ts`, `.cts`, `.mts`, `.jsx`, `.tsx`, `.json`, `.yml`, `.yaml`, `.toml`).
- Skip `*.min.js`, `*.min.css`, `*.map`, lockfiles and SVGs. Minified
  output is legitimately one long line.
- If check 1 already flagged the line, don't add a second finding for it.

## Severity

| Signal | Level | Why |
|---|---|---|
| Code after padding, in a `*.config.*` file | **High** | Runs on every build, hidden from review |
| Code after padding, any other file | **Medium** | Hidden, but not necessarily executed |
| Same-line code after the export, in a `*.config.*` file | **High** | Exactly the PolinRider shape |
| Later-line statement after the export that isn't a declaration | **Low** | Usually a refactor, occasionally a payload |
| Very long line | **Low** | Review prompt only |

Add these rows to the behavioral table in
[docs/severity.md](../severity.md).

## Suggested shape

```rust
pub enum PaddingKind {
    CodeAfterPadding { column: usize, run: usize },
    CodeAfterExport { same_line: bool },
    LongLine { length: usize },
}

pub fn scan_padding(file: &str, source: &str) -> Vec<Finding>;
```

`file` is the repo-relative path with `/` separators. Use it both as the
label and to decide whether this is a `*.config.*` file. Adapt `Finding` to
the type Days 1–2 defined.

## Fixtures

All of these must be defanged. The hidden code is
`console.log("defanged fixture")` or similar, never a real payload. Use
`example.invalid` for any URL.

**Positive (must flag):**
- `postcss.config.js` built in the Trellis shape: an ordinary
  `module.exports = { plugins: { tailwindcss: {}, autoprefixer: {} } };`
  followed on the same line by ~300 spaces and then a defanged
  `global['_x']=…;console.log("defanged")` statement. This is the
  acceptance case. It should produce checks 1 and 2 together.
- `tailwind.config.ts` with `export default { … }` and code after it
  on the same line, with no padding (check 2 only).
- `src/util.js` with code after 200 spaces (check 1, medium).
- The same padded line saved with `\r\n` endings, and the same line padded
  with tabs.

**Negative (must not flag):** the 20 common configs. Use the default
scaffold each tool generates, saved under
`tests/fixtures/repo-scan/padding/clean/`:

| | | | |
|---|---|---|---|
| `postcss.config.js` | `tailwind.config.js` | `tailwind.config.ts` | `vite.config.ts` |
| `webpack.config.js` | `next.config.js` | `next.config.mjs` | `babel.config.js` |
| `jest.config.js` | `vitest.config.ts` | `eslint.config.js` (flat config) | `prettier.config.js` |
| `rollup.config.mjs` | `svelte.config.js` | `astro.config.mjs` | `nuxt.config.ts` |
| `playwright.config.ts` | `tsup.config.ts` | `commitlint.config.js` | `stylelint.config.js` |

Add a short `README.md` next to them that says where each one came from,
for example "output of `npm create vite@5`". Then the next person can
refresh them. Also add these specific negative cases:

- A config that ends with `function helper() {}` after `export default`.
  This is legal and hoisted, so it must not be flagged.
- `module.exports = {…}; module.exports.extra = 1;`.
- A line with a long run of trailing spaces and nothing after it.
- A deeply indented line (~40 spaces) inside a nested object.
- An `export default` whose value contains `}` inside a string and inside a
  template literal. The depth tracker must not end the statement early.
- `dist/app.min.js`, a single 50 kB line. Check 3 must skip it.

## Known traps

- **Self-scan.** Day 1 requires `depradar repo-scan` on this repo to
  return an empty SARIF. The positive fixtures above *will* trip this
  detector. Exclude `tests/fixtures/repo-scan/**` in the repo's own
  `depradar.toml`, or build the positive fixtures as strings inside the
  test. Days 4–7 have the same problem, so settle the approach once and
  record it in [README.md](README.md).
- **Generated files** (`*.generated.ts`, protobuf output) can have long
  lines. Users suppress those with `exclude`. Don't special-case them.
- **Markdown and data files** (`.md`, `.csv`) are out of scope for checks
  1 and 2. Tables and code blocks there legitimately contain long runs of
  spaces.

## Out of scope

- Deobfuscating or decoding the hidden code. Day 4 scores how obfuscated
  it is.
- Matching known PolinRider markers inside the hidden code. Day 2's IOC
  pack already does that, and both findings can appear on the same line.
