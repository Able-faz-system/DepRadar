# Security Policy

DepRadar has two halves with different risk profiles:

- **`contracts/deprader_router`** — a Soroban smart contract that locks and
  pays out bounty funds. A bug here can mean lost or misdirected funds.
- **The off-chain scanner toolkit** (`src/`, `client/`) — reads public
  registry and vulnerability data. A bug here can mean missed or false
  findings, which matters for a tool people rely on to catch supply-chain
  attacks.

Both are in scope, and we ask that vulnerabilities in either be disclosed
responsibly.

## Supported Versions

DepRadar is pre-1.0 and deployed on Stellar **testnet** only. Security fixes
land on `main`; there are no maintained release branches yet.

| Version        | Supported          |
| -------------- | ------------------ |
| `main` (0.1.x) | :white_check_mark: |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report privately through GitHub's private vulnerability reporting: go to
the repository's **Security** tab → **Report a vulnerability**. This opens
an advisory visible only to maintainers.

Please include as much of the following as you can:

- A description of the issue and its impact (e.g. funds released to the
  wrong address, a claim that can be blocked or front-run, a typosquat that
  evades detection, a crash on crafted registry data).
- Steps to reproduce, ideally a failing test against
  `contracts/deprader_router` or the relevant `src/` module.
- The commit you tested against.

## What to Expect

- **Acknowledgement** within 3 business days.
- **Initial assessment** (severity, affected component) within 7 days.
- **Fix and disclosure** coordinated with you. We'll credit you in the
  advisory and in [PROGRESS.md](PROGRESS.md) unless you'd rather stay
  anonymous.

## Scope

In scope:

- `contracts/deprader_router` — authorization, fund custody, claim/release
  state transitions, storage handling.
- `src/` — detection logic (typosquat, CVE matching) and parsing of
  untrusted input (registry metadata, OSV responses, manifests).
- `client/` — the testnet read client.
- CI configuration under `.github/`.

Out of scope:

- Vulnerabilities in third-party dependencies with no DepRadar-specific
  impact — report those upstream (we track them via Dependabot and
  `cargo audit`).
- Findings that require a compromised maintainer machine or leaked
  testnet keys.
- Testnet faucet or network-level issues in Stellar itself.

## Reporting Malicious Packages

If you've found a malicious or typosquatted package *in a public registry*
(rather than a bug in DepRadar), report it to the registry first —
[npm](https://docs.npmjs.com/reporting-malware-in-an-npm-package),
[PyPI](https://pypi.org/security/),
[crates.io](https://crates.io/policies/security). Then feel free to open a
regular issue here so DepRadar's detection can be checked against it.
