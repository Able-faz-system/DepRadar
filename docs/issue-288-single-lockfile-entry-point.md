# Issue #288 — Merge `parse_lockfile` and `parse_file` into One Entry Point

## Problem

`src/lockfile/mod.rs` has two public functions that both turn a path into dependencies by dispatching on `detect()`:

| | `parse_lockfile(&Path)` | `parse_file(impl AsRef<Path>)` |
|---|---|---|
| Returns | `ParsedLockfile { ecosystem, dependencies, warnings }` | `Vec<Dependency>` |
| Error type | `LockfileError` | `ParseError` |
| `package-lock.json` | Parsed via `npm::parse_lock` | **Rejected** with `ParseError::Unsupported` |
| `npm-shrinkwrap.json` | Parsed (special-cased by name) | `UnknownFile`, because `detect()` doesn't know it |
| `requirements*.txt` | Collects warnings | Drops warnings |
| Callers | `src/pipeline.rs`, `tests/lockfile.rs` | Only its own unit tests |

The two match blocks are already out of sync, and every new format has to be added in both places.

## Goal

Keep one dispatch function that handles every `LockfileKind`. If the other one stays, it becomes a thin wrapper with no `match` of its own.

## Affected Files

| File | Change |
|---|---|
| `src/lockfile/mod.rs` | Single dispatch; add `NpmShrinkwrap` handling to `detect`; remove or wrap `parse_file`; update module docs |
| `src/lockfile/mod.rs` (tests) | Add the every-kind test; update the `parse_file_*` tests |
| `PROGRESS.md` | Add entry |

## Design

### Keep `parse_lockfile`

`parse_lockfile` is the richer function. It returns the ecosystem and warnings and is what the pipeline already uses, so keep it as the single entry point. `parse_file` has no callers outside its own tests, so the cleanest option is to delete it. If you'd rather not change the public API in this PR, reduce it to:

```rust
/// Reads `path` and returns its dependencies.
#[deprecated(note = "use `parse_lockfile`, which also returns the ecosystem and warnings")]
pub fn parse_file(path: impl AsRef<Path>) -> Result<Vec<Dependency>, LockfileError> {
    parse_lockfile(path.as_ref()).map(|parsed| parsed.dependencies)
}
```

Either way there must be exactly one `match kind { … }` that selects a parser.

### Move the special cases into `detect`

The `if file_name == "package-lock.json" || … "npm-shrinkwrap.json"` short-circuit exists only because `detect()` doesn't cover shrinkwrap. Make `detect` the single source of truth instead:

```rust
"package-lock.json" | "npm-shrinkwrap.json" => LockfileKind::PackageLock,
```

(Or add a separate `NpmShrinkwrap` variant if you need to tell them apart. They share a format, so one variant is enough.)

### One exhaustive match

Give each arm both the parser and the ecosystem so the two can't drift apart, and leave out the `_ =>` and `unreachable!()` arms. That way the compiler flags any new `LockfileKind` that isn't handled:

```rust
pub fn parse_lockfile(path: &Path) -> Result<ParsedLockfile, LockfileError> {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let kind = detect(path).ok_or_else(|| LockfileError::UnknownFormat(file_name.into()))?;
    let contents = fs::read_to_string(path).map_err(LockfileError::Io)?;
    let invalid = |err: ParseError| LockfileError::Invalid(err.to_string());

    let (ecosystem, dependencies, warnings) = match kind {
        LockfileKind::PackageLock => return npm::parse_lock(&contents),
        LockfileKind::Requirements => {
            let (deps, warnings) =
                pypi::parse_requirements_with_warnings(&contents).map_err(invalid)?;
            (Ecosystem::PyPI, deps, warnings)
        }
        LockfileKind::PipfileLock => (Ecosystem::PyPI, pypi::parse_pipfile_lock(&contents).map_err(invalid)?, vec![]),
        LockfileKind::PoetryLock  => (Ecosystem::PyPI, pypi::parse_poetry_lock(&contents).map_err(invalid)?, vec![]),
        LockfileKind::CargoLock   => (Ecosystem::CratesIo, cargo::parse_cargo_lock(&contents).map_err(invalid)?, vec![]),
        LockfileKind::PackageJson => (Ecosystem::Npm, npm::parse_package_json(&contents).map_err(invalid)?, vec![]),
        LockfileKind::YarnLock    => (Ecosystem::Npm, yarn::parse_yarn_lock(&contents).map_err(invalid)?, vec![]),
        LockfileKind::PnpmLock    => (Ecosystem::Npm, pnpm::parse_pnpm_lock(&contents).map_err(invalid)?, vec![]),
    };
    Ok(ParsedLockfile { ecosystem, dependencies, warnings })
}
```

Behaviour change to note: the old code checked the name before reading the file, so an unknown name never touched the disk. The new code keeps that order (`detect` runs before `read_to_string`).

### `ParseError::Unsupported`

Once `PackageLock` is handled, nothing constructs `ParseError::Unsupported`. Remove the variant (it's `pub`, so mention that in PROGRESS.md), or keep it if per-format parsers still use it. Run `grep -rn Unsupported src/` to check.

## Acceptance Criteria Mapping

### One fixture per `LockfileKind`, through the single entry point

All the fixtures already exist under `tests/fixtures/`:

| `LockfileKind` | Fixture |
|---|---|
| `PackageLock` | `tests/fixtures/npm-basic/package-lock.json` |
| `YarnLock` | `tests/fixtures/yarn.lock` |
| `PnpmLock` | `tests/fixtures/pnpm-lock-v9.yaml` (copy into a dir as `pnpm-lock.yaml`, since `detect` matches on the exact name) |
| `Requirements` | `tests/fixtures/pypi-basic/requirements.txt` |
| `PoetryLock` | `tests/fixtures/poetry.lock` |
| `PipfileLock` | `tests/fixtures/Pipfile.lock` |
| `CargoLock` | `Cargo.lock` (repo root) or `tests/fixtures/cargo/` |
| `PackageJson` | `tests/fixtures/package.json` |

Check the file names before relying on them. `detect` matches exact names, so a fixture named `pnpm-lock-v9.yaml` won't be detected. Use `tempfile` (already a dev-dependency) to copy it to a correctly named path.

```rust
#[test]
fn parse_lockfile_handles_every_kind() {
    use crate::model::Ecosystem::*;
    let cases: &[(LockfileKind, &str, Ecosystem)] = &[
        (LockfileKind::PackageLock,  "tests/fixtures/npm-basic/package-lock.json", Npm),
        (LockfileKind::YarnLock,     "tests/fixtures/yarn.lock",                   Npm),
        // … one row per variant …
    ];
    for (kind, path, ecosystem) in cases {
        let path = repo_path(path);
        assert_eq!(detect(&path), Some(*kind), "{path:?}");
        let parsed = parse_lockfile(&path).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
        assert_eq!(parsed.ecosystem, *ecosystem, "{kind:?}");
        assert!(!parsed.dependencies.is_empty(), "{kind:?} parsed no dependencies");
    }
}
```

To make sure the table can't silently miss a variant, add a helper with an exhaustive `match` over `LockfileKind` that returns its fixture. The compiler will then reject any new variant without one.

### CI

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Implementation Checklist

- [ ] `detect` maps `npm-shrinkwrap.json` to a `LockfileKind`
- [ ] `parse_lockfile` has one exhaustive `match` with no `_` or `unreachable!`
- [ ] `parse_file` deleted, or reduced to a one-line wrapper
- [ ] `ParseError::Unsupported` removed if unused
- [ ] Module doc comment (`//! … [parse_file] reads a file …`) updated
- [ ] `parse_lockfile_handles_every_kind` test added
- [ ] Old `parse_file_*` tests ported to `parse_lockfile`
- [ ] PROGRESS.md entry, PR description includes `Closes #288`

## Notes

- `src/pipeline.rs` only calls `parse_lockfile`, so the pipeline needs no change.
- Check `crates/` (e.g. the wasm crate) and `fuzz/` for uses of `parse_file` before deleting it: `grep -rn parse_file crates fuzz benches`.
- Don't fold `LockfileError` and `ParseError` into one type here. That's a bigger change than the issue asks for.
