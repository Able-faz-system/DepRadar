# Contributing

Thanks for helping make DepRadar better. Start with an open issue labelled
`good first issue` or `help wanted`, comment that you're picking it up, and
open a PR that references it.

## Development Setup

Prerequisites: a stable Rust toolchain, the `wasm32v1-none` target (for the
contract), and Node.js 22+ (for `client/`).

```bash
rustup target add wasm32v1-none

# Off-chain toolkit + contract unit tests
cargo test --workspace --locked

# Lints — CI runs both and fails on any warning
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings

# Build the contract to wasm
cargo build -p deprader-router --target wasm32v1-none --release --locked

# Try the typosquat scanner
cargo run --bin scan -- expresss

# Testnet read client
cd client && npm ci && npm run typecheck && npm start
```

Some tests in `src/cve/` call the live [OSV API](https://osv.dev); they skip
(rather than fail) when the network is unavailable.

## Pull Requests

- One issue per PR where possible, with `Closes #N` in the description.
- Keep CI green: fmt, clippy, tests, the wasm build, and the client
  typecheck all run on every PR.
- If you change `contracts/`, commit the regenerated files under
  `contracts/deprader_router/test_snapshots/`.
- Security-sensitive findings go through [SECURITY.md](SECURITY.md), not
  public issues.

## Closing Issues

No issue is closed without an entry in [PROGRESS.md](PROGRESS.md)
describing what was built, by whom, and which issue it closes. This is
called out explicitly in the Definition of Done on the issue templates
(`.github/ISSUE_TEMPLATE/`), but it applies to any issue, templated or not.

## Testnet Identities

Don't share or commit Stellar secret keys, including testnet ones. Generate
your own local identity instead:

```bash
stellar keys generate <name> --network testnet --fund
```

This creates a keypair under your local Stellar CLI config (see
`.gitignore` — `.stellar/`, `.soroban/`, and `**/identity/*.toml` are all
excluded from the repo) and funds it via friendbot. Use `<name>` wherever
these docs or scripts expect a `--source` or `--admin` identity.

The testnet admin identity used for the current `deprader_router` deployment
(`GD7IF4QGOEOG6Q7WBD6GDZEZ7HAY4RLDJ5QQTRMJRHOBR43DI3YNJNM6`) will be rotated
before this repo goes public, since its address is already exposed on-chain
through the deploy and `initialize` transactions. Testnet funds have no real
value, but treat the rotation as a reminder not to reuse identities across
public and private stages of a project.
