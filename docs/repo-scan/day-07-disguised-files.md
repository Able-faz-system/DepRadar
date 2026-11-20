# Day 7 — Disguised-file detector

**Issue:** [#295](https://github.com/DepRadarOrg/DepRadar/issues/295) ·
**Depends on:** Day 6 (merged first) ·
**Relevant file:** `src/behavioral/binaries.rs`

## Goal

Find files in a repository that pretend to be something harmless. The
typical case is a JavaScript payload renamed to look like a web font, so
that it slips past both reviewers and tools that skip "assets".

**Acceptance test:** a JS file renamed to `fa-solid-500.woff2` is caught.

## What to detect

There are three independent checks. Each one produces its own finding
kind, so that a report says *why* a file was flagged.

### 1. Fonts whose magic bytes aren't a font

For any file with a font extension, read the first few bytes and check
them against the format's signature.

| Extension | Expected leading bytes |
|---|---|
| `.woff` | `77 4F 46 46` (`wOFF`) |
| `.woff2` | `77 4F 46 32` (`wOF2`) |
| `.ttf` | `00 01 00 00` or `74 72 75 65` (`true`) |
| `.otf` | `4F 54 54 4F` (`OTTO`) |
| `.ttc` | `74 74 63 66` (`ttcf`) |
| `.eot` | magic `0x504C` (`LP`) at byte offset 34 |

- Match extensions case-insensitively (`.WOFF2` counts).
- Treat a file that is too short to contain the signature as a mismatch.
- *Optional:* if the mismatched content is mostly printable ASCII or UTF-8,
  note that it "looks like text/script". This raises confidence but isn't
  required to flag the file.
- Only read the header bytes. Don't load whole files into memory.

### 2. Known suspicious names

These names come from the campaign this roadmap targets. Keep them in one
`const` list so they're easy to extend.

| Pattern | Notes |
|---|---|
| `*.llf` | Any file with the `.llf` extension |
| `dist/setup.js` | Exact path match, relative to the repo root |
| `temp_auto_push.bat` | Filename match at any depth |
| `config.bat` listed in `.gitignore` | Parse `.gitignore` files and flag an entry that ignores `config.bat`. The ignore rule itself is the signal, because it hides a dropper from `git status`. It fires even if the file isn't present. |

Match paths with `/` separators, relative to the scan root, so results are
the same on Windows.

### 3. Native binaries

Use the existing `behavioral::binaries` detection. **Don't add a second
magic-byte table.** If Day 6 (or earlier) didn't land that module, raise
it on the issue before you write your own. For reference, it should
recognise ELF (`7F 45 4C 46`), PE (`4D 5A`), and Mach-O (`FE ED FA CE/CF`,
`CE/CF FA ED FE`).

One trap: `CA FE BA BE` is both a Mach-O universal binary and a Java
`.class` file. Don't count `.class` files as disguised native binaries.

The disguise check is: *a native binary whose extension claims a
non-binary type* (for example `.png`, `.woff2`, `.txt`, `.js`). A binary
named `.exe`, `.so`, `.dll` or `.dylib` may still be worth reporting
through `behavioral::binaries`, but that is not a *disguised* file.

## Suggested shape

```rust
pub enum DisguiseKind {
    FontMagicMismatch { ext: String, looks_like_text: bool },
    KnownSuspiciousName { pattern: &'static str },
    GitignoreHidesFile { entry: String },
    DisguisedNativeBinary { format: BinaryFormat, claimed_ext: String },
}

pub fn scan_disguised_files(root: &Path) -> Vec<Finding>;
```

Adapt this to the `Finding` type the earlier days defined.

## Fixtures

Store these under `tests/fixtures/disguised/`, or build them inline as byte
arrays. All of them must be defanged.

**Positive (must flag):**
- `fa-solid-500.woff2` whose content is `console.log("defanged fixture");`.
  This is the acceptance case.
- `icon.ttf` that is empty or 2 bytes long (too short).
- `data.llf` with any content.
- `dist/setup.js`, `temp_auto_push.bat`.
- `.gitignore` containing `config.bat`.
- `logo.png` that starts with `7F 45 4C 46` followed by padding. This is
  only the header bytes, not a working binary.

**Negative (must not flag):**
- Minimal real font headers: a `.woff2` starting `wOF2…`, a `.ttf`
  starting `00 01 00 00…`, and an `.otf` starting `OTTO…`. Header bytes
  plus padding are enough.
- `src/setup.js` (the right name in the wrong place), `setup.bat`, and
  `config.bat.example`.
- `.gitignore` containing `config.json`, `*.log` and `node_modules/`.
- A `.class` file starting `CA FE BA BE`.

## Out of scope

- Decoding or deobfuscating the payload. Detection only.
- Formats other than fonts, for now. The code should make it easy to add
  more magic tables later (images, PDFs).
