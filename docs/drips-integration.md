# Drips Integration Design

Status: **Draft — awaiting maintainer review**
Issue: [#146](https://github.com/DepRadarOrg/DepRadar/issues/146)
Last updated: 2025-01-01

> **Review gate:** Per the acceptance criteria for #146, this document must be
> reviewed and approved by a maintainer **before any implementation issue is
> opened**. Do not start coding against any of the designs below until this
> doc has been signed off.

---

## 1. Background

The DepRadar mission states that bounty payouts are routed through
[Drips](https://www.drips.network/). Today, the router contract
(`contracts/deprader_router/src/lib.rs`) pays a claimant directly: it transfers
the bounty amount from the contract's balance to the claimant's address in a
single transfer. There is currently **no design** describing how the router and
Drips connect, what Drips actually offers on Stellar, or which of the possible
integration shapes we should pursue.

This document:

1. Summarises what Drips offers on Stellar (and what it does not).
2. Lays out three candidate designs.
3. Compares their trade-offs.
4. Makes a recommendation.
5. Lists the concrete contract changes each design would require in
   `contracts/deprader_router/src/lib.rs`.

---

## 2. What Drips offers on Stellar

Drips is a protocol for **continuous, streaming payments** and **splitting**
incoming funds across a set of recipients according to configured weights. On
EVM chains the core primitives are:

- **Drips accounts** — an address that can receive streams and hold a balance.
- **Streams** — a per-second flow of a token from a sender to a receiver,
  funded up-front by a balance the sender tops up.
- **Splits** — a receiver can configure a weighted list of sub-recipients; any
  funds that arrive are split across that list (recursively).
- **Collect** — a receiver (or anyone on their behalf) can "sweep" accrued
  stream/split balances into their wallet.

### 2.1 Stellar specifics and caveats

Stellar's execution model differs from the EVM in ways that matter here:

- **Soroban smart contracts** are the execution environment. Any Drips
  integration must be a Soroban contract (or a set of them), not a port of the
  EVM bytecode.
- **Assets are trustline-based.** A recipient must have a trustline for a
  non-native asset before it can hold it. Native XLM needs no trustline.
  Any design that credits a Drips account must account for trustline setup.
- **No native "per-second" clock primitive.** Streams are emulated by storing a
  rate and a start ledger/timestamp and computing accrued amounts on read or on
  `collect`. This is the same approach the EVM Drips contracts use, but the
  time source on Soroban is the ledger timestamp, so accrual granularity is
  bounded by ledger close time.
- **Authorization model.** Soroban uses `require_auth` on addresses. A stream
  sender must authorise the funding transfer; a `collect` on behalf of a
  receiver must be authorised by that receiver (or the contract must be
  designed so that anyone can trigger a sweep to the receiver's own address,
  which is safe because funds only ever move *to* the receiver).

> **Verification note:** The exact set of Drips contracts deployed on Stellar
> (mainnet/testnet contract IDs) and whether an official Soroban port exists at
> the time of writing must be confirmed with the Drips team before
> implementation. Links to check:
>
> - Drips docs: <https://docs.drips.network/>
> - Drips protocol overview: <https://www.drips.network/>
> - Soroban docs: <https://developers.stellar.org/docs/smart-contracts>
>
> If no official Soroban Drips deployment exists, designs 2 and 3 below require
> us to either (a) depend on a third-party port, or (b) implement a minimal
> Drips-compatible splits/streams module ourselves. This is the single biggest
> open question and should be resolved during review.

---

## 3. Candidate designs

Throughout, "the router" refers to `DepRadarRouter` in
`contracts/deprader_router/src/lib.rs`, and "claimant" refers to the address
that is entitled to a bounty payout.

### Design A — Claimant receives directly, then splits

The router keeps its current behaviour: on a successful claim it transfers the
full bounty to the claimant's address. Splitting is entirely the claimant's
responsibility — the claimant configures their own Drips splits so that funds
landing in their account are forwarded to their team, a treasury, etc.

**Flow**

```
router ──(full bounty)──▶ claimant ──(Drips splits)──▶ sub-recipients
```

**Trade-offs**

- ✅ Smallest change to the router; essentially zero new contract surface.
- ✅ No dependency on a Soroban Drips deployment inside the payout path, so
  payouts never block on Drips availability.
- ✅ Claimant keeps full custody and control of split configuration.
- ❌ Does not actually "route payouts through Drips" from the protocol's point
  of view — Drips is opt-in and off the critical path. Weakest match to the
  mission statement.
- ❌ No protocol-level guarantee that a team/treasury share is honoured.
- ❌ Claimant must set up trustlines and Drips config themselves; poor UX for
  first-time claimants.

### Design B — Router sends a share to a project's Drips list

On claim, the router splits the bounty: a configurable percentage (or fixed
amount) is sent to the project's Drips account/list, and the remainder goes to
the claimant. The project's Drips splits then distribute that share to
maintainers, a treasury, etc.

**Flow**

```
                 ┌──(claimant share)──▶ claimant
router ──────────┤
                 └──(project share)───▶ project Drips account ──(splits)──▶ maintainers/treasury
```

**Trade-offs**

- ✅ Directly satisfies "payouts go through Drips" for the project share.
- ✅ Project-level split policy is enforced by the protocol, not by goodwill.
- ✅ Claimant still gets a direct, immediate payout for their share.
- ⚠️ Requires the router to know each project's Drips account and split ratio.
  This is new per-project configuration state.
- ⚠️ Requires a Soroban Drips (or Drips-compatible) contract to exist and be
  callable from the router. If it is unavailable, the project share must be
  held or fall back to a direct transfer — a fallback path we must design.
- ⚠️ Trustline requirements: the project Drips account must hold a trustline
  for the bounty asset, or the transfer fails. The router must handle a
  missing-trustline error gracefully.
- ❌ Two transfers per claim instead of one — slightly higher fees and more
  failure modes.

### Design C — Drips streams fund the bounty pool

Instead of splitting at payout time, the router's bounty pool is **funded by
Drips streams**. Sponsors stream tokens into the router's (or a dedicated
treasury) Drips account; the router draws from that balance to pay bounties.
Payouts to claimants remain direct transfers.

**Flow**

```
sponsors ──(Drips streams)──▶ router/treasury Drips account ──(withdraw)──▶ bounty pool
bounty pool ──(full bounty)──▶ claimant
```

**Trade-offs**

- ✅ Strong fit for *recurring* funding (grants, sponsorships) and gives
  sponsors a familiar Drips UX.
- ✅ Payout path to the claimant is unchanged — low risk to the core claim flow.
- ⚠️ Requires the router (or a treasury contract) to be able to `collect` from
  a Drips stream, which means the router must be authorised as a stream
  receiver and must handle accrual/withdrawal.
- ⚠️ Introduces a dependency on Drips for *solvency*: if the stream contract is
  unavailable, the router cannot top up its pool. (Existing balances remain
  spendable, so payouts are not blocked — only funding is.)
- ❌ Does not route *payouts* through Drips; it routes *funding* through Drips.
  Only a partial match to the mission statement.
- ❌ Most new moving parts (stream accounting, withdrawal logic, treasury
  separation) for the least direct benefit to the stated goal.

---

## 4. Comparison

| Criterion                         | A: direct + split | B: router → Drips list | C: streams fund pool |
| --------------------------------- | :---------------: | :--------------------: | :------------------: |
| Matches "payouts via Drips"       |        Low        |         High           |        Medium        |
| Router contract change size       |      Small        |         Medium         |        Medium        |
| Depends on Soroban Drips at payout|        No         |          Yes           |         No           |
| Enforces project/team share       |        No         |          Yes           |         No           |
| Claimant UX (first claim)         |       Poor        |          Good          |        Good          |
| Failure modes added to claim path |       None        |         Some           |        None          |
| Enables recurring sponsor funding |        No         |          No            |         Yes          |

---

## 5. Recommendation

**Adopt Design B as the target, shipped in two phases, with Design A as the
interim behaviour.**

Rationale:

1. Design B is the only option that actually routes *payouts* through Drips,
   which is what the mission promises.
2. It keeps a direct claimant payout, so the claimant is never blocked on
   Drips availability — only the project share depends on it.
3. It is a strict superset of Design A: with the project share set to 0% (the
   default), the router behaves exactly as it does today. This lets us ship the
   configuration and fallback plumbing first and turn on Drips routing per
   project once a Soroban Drips deployment is confirmed.

**Phase 1 (interim, no Drips dependency):** implement the per-project split
configuration and the two-transfer payout path, with the project share sent
directly to the project's configured address. This is Design A behaviour plus
the plumbing Design B needs.

**Phase 2 (Drips routing):** once a Soroban Drips (or Drips-compatible)
contract is confirmed and audited, point the project-share transfer at the
project's Drips account so its splits apply.

Design C is a good *future* complement (recurring sponsor funding) but should
not block the payout-routing work and is out of scope for this issue.

**Open questions for the maintainer to resolve during review:**

- Does an official Soroban Drips deployment exist? If not, do we depend on a
  third-party port or implement a minimal splits module?
- Should the project share be a percentage or a fixed amount? Percentage is
  more flexible; fixed amount is simpler to reason about.
- What is the fallback when the project Drips account lacks a trustline for
  the bounty asset — hold the share in escrow, or send it directly to the
  project address?
- Who is authorised to set a project's Drips account and split ratio?

---

## 6. Contract changes required per design

All changes are in `contracts/deprader_router/src/lib.rs` unless noted.

### Design A — Claimant receives directly, then splits

- **No changes required.** The current single-transfer payout already
  implements this design.
- Optional: add a doc comment on the payout function noting that claimants may
  configure Drips splits on their own account.

### Design B — Router sends a share to a project's Drips list

New storage (instance or persistent, keyed by project id):

- `ProjectDripsConfig { drips_account: Address, share_bps: u32 }` — the
  project's Drips account and the share of each bounty (in basis points) routed
  to it. `share_bps` must be `<= 10_000`.

New / changed entry points:

- `set_project_drips_config(env, project_id, drips_account, share_bps)` —
  admin/owner-authorised; validates `share_bps <= 10_000` and that
  `drips_account` is a valid address. Emits an event.
- `get_project_drips_config(env, project_id) -> Option<ProjectDripsConfig>` —
  read-only view.
- **Modify the claim/payout function** to:
  1. Read the project's `ProjectDripsConfig` (default to `share_bps = 0` if
     unset, preserving current behaviour).
  2. Compute `project_share = amount * share_bps / 10_000` and
     `claimant_share = amount - project_share` (compute the project share first
     and subtract to avoid rounding loss).
  3. Transfer `claimant_share` to the claimant (existing path).
  4. If `project_share > 0`, transfer it to `drips_account` (Phase 1) or call
     the Drips contract to credit the project's Drips account (Phase 2).
  5. Handle a failed project-share transfer (e.g. missing trustline) according
     to the fallback decided in review — do **not** leave the claim in a
     half-paid state; either revert the whole claim or record the held share.
- Add a `ProjectShareRouted` event carrying `(project_id, drips_account,
  project_share, claimant_share)`.

Cross-contract:

- If Phase 2 calls a Drips contract, add its address to the router's
  constructor/initialiser and store it; add the Drips client interface
  (generated from its Soroban spec) to the router's dependencies.

### Design C — Drips streams fund the bounty pool

New storage:

- `drips_stream_source: Address` — the Drips account/contract the router
  collects from (set at init, admin-updatable).

New / changed entry points:

- `collect_stream_funding(env)` — permissionless (or admin) entry point that
  calls the Drips contract to `collect` accrued stream balance into the
  router's bounty pool, then updates the pool balance. Must be idempotent and
  safe to call when nothing has accrued.
- `get_stream_funding_accrued(env) -> i128` — read-only view of what is
  currently collectable (for monitoring/UX).
- **Modify the payout function** only insofar as it reads the pool balance;
  the claimant transfer itself is unchanged. Ensure the pool balance reflects
  collected stream funds.
- Add a `StreamFundingCollected` event carrying `(amount)`.

Cross-contract:

- Add the Drips contract address to the router's initialiser and store it.
- Add the Drips client interface to the router's dependencies.
- The router must be authorised as a stream receiver on the Drips side; this is
  configured in Drips, not in the router, but must be documented in the
  deployment runbook.

---

## 7. Verification checklist

Before this doc is considered final, confirm:

- [ ] Every link in this document resolves (Drips docs, Drips site, Soroban
      docs).
- [ ] The existence (or absence) of a Soroban Drips deployment is confirmed and
      recorded in §2.1.
- [ ] Any commands referenced in follow-up implementation issues have been run
      and verified (this doc contains no executable commands; build/test
      commands live in `CONTRIBUTING.md`).
- [ ] A maintainer has reviewed and approved this document.
- [ ] `PROGRESS.md` has an entry for this issue.
- [ ] The PR description includes `Closes #146`.

---

## 8. References

- Issue #146: <https://github.com/DepRadarOrg/DepRadar/issues/146>
- Contributing guide: <https://github.com/DepRadarOrg/DepRadar/blob/main/CONTRIBUTING.md>
- Drips documentation: <https://docs.drips.network/>
- Drips website: <https://www.drips.network/>
- Stellar Soroban documentation: <https://developers.stellar.org/docs/smart-contracts>
- Router contract: `contracts/deprader_router/src/lib.rs`