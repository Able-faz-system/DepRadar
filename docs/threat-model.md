# Threat Model — Bounty Router

This document describes the security properties of the DepRadar Bounty Router contract, what it protects against, and what it relies on.

## Scope

This threat model covers the Bounty Router smart contract (`contracts/deprader_router/`) deployed on Stellar. It does not cover the off-chain scanner (typosquat detection, CVE matching) or the dashboard. For scanner and crawler vulnerabilities, see [SECURITY.md](../SECURITY.md).

## Actors

| Actor | Role | Trust Level |
|-------|------|-------------|
| **Funder/Creator** | Locks funds against a bounty | Untrusted — can be malicious or compromised |
| **Researcher/Claimant** | Submits a claim with proof | Untrusted — can submit junk claims |
| **Admin** | Reviews claims, releases or rejects | **Trusted** — chosen at `initialize()` time |
| **Contract** | Enforces rules, holds funds | Trusted — correctness assumed |
| **Stellar Network** | Executes transactions, stores state | Trusted — consensus assumed correct |

The admin is the only trusted party. The contract design assumes the admin acts honestly and reviews claims correctly. A compromised admin can steal locked funds.

## Assets

| Asset | Owner | Protection Goal |
|-------|-------|----------------|
| **Locked bounty funds** | Creator, held by contract | Only released to a valid claimant after admin review, or refunded to creator after deadline |
| **Claim priority** | Researcher | First valid claim wins, junk claims don't block real researchers |
| **Contract state** | All users | State transitions follow rules (e.g. can't release twice, can't cancel a paid bounty) |

## Trust Assumptions

1. **The admin reviews claims honestly.** A malicious admin can release any claim regardless of proof quality, or reject valid claims to favor an accomplice.
2. **The admin's signing key is secure.** A compromised admin key allows an attacker to drain all locked bounties.
3. **The Stellar network and Soroban runtime are correct.** The contract assumes the ledger is not rolled back, transactions are atomic, and storage is persistent.
4. **Tokens used for bounties are legitimate.** A malicious token contract can refuse transfers on release, locking funds.
5. **Time is monotonic.** The contract relies on `env.ledger().timestamp()` for deadlines and review windows.

## Known Threats & Mitigations

### 1. Claim Squatting (Front-Running)

**Threat:** A malicious researcher monitors the mempool, sees a bounty creation, and immediately submits a junk claim to block the real finder.

**Mitigation:** ✅ **Mitigated**. The contract allows multiple claims per bounty (up to 50). The admin can reject junk claims while leaving the bounty open for others. A junk claim submitted first does not prevent a later valid claim from being paid.

**Test coverage:** `test_multiple_claimants_can_claim_one_bounty`, `test_pending_claim_holds_funds_until_review_window_ends`.

**Related issues:** N/A — fixed in v2 router rewrite.

### 2. Admin Compromise

**Threat:** An attacker gains control of the admin's signing key and uses `release(bounty_id, claim_id)` to pay out to their own address, draining all locked bounties.

**Mitigation:** ⚠️ **Partially mitigated**. The contract enforces a two-step admin rotation (`propose_admin` / `accept_admin`), so a key compromise can be detected before full rotation. However, **a compromised current admin can still release any bounty immediately**. This is an inherent limitation of the single-admin design.

**Open work:** Multi-sig admin (e.g. 2-of-3 review quorum) would limit single-key compromise, but is not implemented. See #TBD for the proposal.

**Test coverage:** `test_admin_rotation_is_two_step`, `test_release_and_reject_errors` (only admin can release).

**Operational mitigation:** Admin key held in a hardware wallet with manual approval for every `release` tx. Testnet admin key rotation after v1 exposure.

### 3. Malicious Token Contract

**Threat:** A bounty is created with a token whose contract can selectively refuse transfers. When the admin releases the bounty, the token refuses the transfer to the claimant, locking the funds.

**Mitigation:** ⚠️ **Not mitigated in contract**. The Bounty Router calls `token.transfer(from, to, amount)` and assumes it succeeds. A malicious token can:
- Refuse the `release` transfer, leaving the bounty stuck in "released" state with funds still locked.
- Refuse the `cancel_bounty` refund, preventing the creator from recovering funds.

**Open work:** Token allowlist — admin can mark tokens as trusted, and `create_bounty` rejects disallowed tokens. See [#121](https://github.com/DepRadarOrg/DepRadar/issues/121), [#122](https://github.com/DepRadarOrg/DepRadar/issues/122).

**Operational mitigation:** Dashboard and documentation should warn funders to only use well-known tokens (native XLM, USDC, established assets). Testnet uses native XLM Stellar Asset Contract.

### 4. Griefing via Claim Spam

**Threat:** A malicious actor submits 50 junk claims on every bounty, filling the claim cap and forcing the admin to review 50 entries per bounty.

**Mitigation:** ✅ **Mitigated**. The contract caps claims at `MAX_CLAIMS_PER_BOUNTY = 50`. The cap prevents unbounded storage growth and review cost. Each address can only claim once per bounty (enforced by `has_address_claimed`).

**Cost:** A griefing attack costs the attacker gas for 50 `submit_claim` transactions per bounty. The admin's review cost is bounded at 50 claims.

**Test coverage:** `test_multiple_claimants_can_claim_one_bounty` (hit the cap), `test_create_bounty_locks_funds_and_assigns_ids` (duplicate claim from same address rejected).

**Open work:** On-chain reputation or a small claim bond (refunded on valid claim, burned on rejection) could raise the cost of spam. Not implemented.

### 5. Storage Expiry (Stellar TTL)

**Threat:** On Stellar, persistent storage expires if not renewed. A bounty's state could be archived mid-review, making it unreadable or unclaimable.

**Mitigation:** ✅ **Mitigated**. The contract extends TTL on every state-modifying call (`extend_instance` and `extend_persistent` in `create_bounty`, `submit_claim`, `reject_claim`, `release`, `cancel_bounty`). The extension is ~100 days (`LEDGER_BUMP_AMOUNT = 518400` ledgers ≈ 30 days, `LEDGER_BUMP_CONSTANT = 1036800` ≈ 60 days for instance storage). Active bounties are never archived.

**Risk:** A bounty created, never claimed, and never cancelled could expire after 60 days. The creator would need to call a read method or `cancel_bounty` to restore it, but `cancel_bounty` itself would fail if the state is archived.

**Open work:** Off-chain TTL monitoring (a cron job that bumps TTL for all active bounties) would eliminate this risk. Not implemented.

**Test coverage:** TTL is extended in tests, but no test validates expiry behavior (Soroban test framework limitations).

### 6. Creator Rug-Pull (Early Cancel)

**Threat:** A creator locks funds in a bounty, a researcher submits a valid claim, and the creator immediately cancels to avoid paying.

**Mitigation:** ✅ **Mitigated**. The contract enforces `cancel_bounty` authorization rules:
- The **admin** can cancel any time (trusted party).
- The **creator** can only cancel if:
  1. The deadline has passed, AND
  2. No claims are pending (all rejected or none submitted), OR
  3. A 7-day review window after the deadline has also passed.

Once a claim is submitted before the deadline, the creator cannot cancel until the admin rejects it or the review window expires.

**Test coverage:** `test_creator_cannot_cancel_before_deadline`, `test_creator_can_cancel_after_deadline_with_no_pending_claims`, `test_admin_can_cancel_any_time`.

### 7. Double-Release

**Threat:** A bug in the contract or a transaction replay allows `release` to be called twice on the same bounty, paying out twice.

**Mitigation:** ✅ **Mitigated**. The contract checks `bounty.status` before releasing. Once `release` sets `status = BountyStatus::PaidOut`, a second `release` fails with `BountyAlreadyClosed`. Each bounty can only be released once.

**Test coverage:** `test_release_and_reject_errors` (release closed bounty rejected).

### 8. Integer Overflow / Underflow (Bounty Amount)

**Threat:** Arithmetic overflow in `amount` calculations (e.g. `create_bounty` with `u64::MAX`, or multiple funders adding to a bounty) could wrap and lock incorrect amounts.

**Mitigation:** ✅ **Mitigated**. Rust checks for overflow in debug builds, and the contract uses checked arithmetic where addition is needed (v2 removed the `fund_bounty` feature). `create_bounty` takes `amount: i128` (Stellar native type for token amounts), which is validated by the token contract on `transfer`.

**Test coverage:** No explicit overflow tests (Rust panics on overflow in test mode).

### 9. Claim Proof Forgery

**Threat:** A researcher submits a claim with a fake `proof_ref` (a PR or writeup they didn't author, or one that doesn't demonstrate the finding).

**Mitigation:** ⚠️ **Not mitigated by contract**. The contract stores `proof_ref` as an opaque string and never validates it. **Proof verification is the admin's responsibility.** A malicious or lazy admin could release a claim without checking the proof.

**Operational mitigation:** The admin must manually verify each `proof_ref` (visit the URL, confirm authorship, check the fix or disclosure). This is the admin's core responsibility.

**Open work:** On-chain proof attestation (e.g. a GitHub bot signs a proof hash and the contract checks the signature) could reduce trust in the admin, but is out of scope for a testnet MVP.

### 10. Deadline Manipulation

**Threat:** A creator sets a deadline in the past, so claims are never accepted, or far in the future (past `MAX_DEADLINE_DURATION = 31536000` seconds = 1 year) to lock funds indefinitely.

**Mitigation:** ✅ **Mitigated**. The contract validates deadlines in `create_bounty`:
- `deadline` must be after the current ledger timestamp.
- `deadline` must be within one year (`deadline - now <= MAX_DEADLINE_DURATION`).

A deadline in the past or too far out is rejected with `InvalidDeadline`.

**Test coverage:** `test_create_bounty_rejects_bad_input`.

### 11. Unauthorized Admin Rotation

**Threat:** An attacker calls `propose_admin` to rotate the admin without the current admin's permission.

**Mitigation:** ✅ **Mitigated**. `propose_admin` requires the current admin's authorization (`env.require_auth(&current_admin)`). Only the admin can propose a new admin. The new admin must then call `accept_admin` to complete the rotation.

**Test coverage:** `test_admin_rotation_is_two_step`.

### 12. Reentrancy via Token Callback

**Threat:** A malicious token contract calls back into the Bounty Router during a `transfer` call (e.g. in `release`), triggering a reentrant `release` or `cancel_bounty` call before the first one completes.

**Mitigation:** ✅ **Mitigated by design**. The contract updates state *before* calling `token.transfer`:
- In `release`, `bounty.status = BountyStatus::PaidOut` is written before the transfer.
- In `cancel_bounty`, `bounty.status = BountyStatus::Refunded` is written before the transfer.

A reentrant call would see the updated status and fail with `BountyAlreadyClosed`.

**Note:** Soroban's execution model may prevent reentrancy entirely (need to confirm), but the contract is written to be safe regardless.

**Test coverage:** Not explicitly tested (would require a malicious token contract in the test environment).

## Out of Scope

- **Scanner false positives/negatives** — the scanner is off-chain and not covered by this threat model.
- **Dashboard compromise** — the dashboard has no authority and can't trigger contract actions.
- **Drips payout integration** — not yet implemented; future threat model needed.
- **Mainnet deployment** — this threat model covers testnet; mainnet would require audit.

## Responsible Disclosure

If you discover a vulnerability in the Bounty Router or any other DepRadar component, please report it privately via GitHub Security Advisories. See [SECURITY.md](../SECURITY.md) for details.

## References

- Contract source: [`contracts/deprader_router/src/lib.rs`](../contracts/deprader_router/src/lib.rs)
- Contract tests: [`contracts/deprader_router/src/test.rs`](../contracts/deprader_router/src/test.rs)
- Live testnet contract: [`CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ`](https://stellar.expert/explorer/testnet/contract/CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ)
- [Security policy](../SECURITY.md)
- [Deployment guide](../DEPLOYMENT.md)
