# Drips Integration: Funding the Bounty Pool from a Drips Stream

This document describes the prototype that lets a project funded on
[Drips](https://www.drips.network/) direct part of that funding into a security
bounty pool automatically. The pool receives Drips streams, and the pool admin
can open bounties against the accumulated pool balance.

> **Status:** prototype. The design below is implemented in `contracts/` and is
> intended to be validated on testnet before any mainnet rollout. See
> [Limitations](#limitations) and [Next steps](#next-steps).

## Contents

- [Design overview](#design-overview)
- [How Drips streams feed the pool](#how-drips-streams-feed-the-pool)
- [Opening bounties from the pool balance](#opening-bounties-from-the-pool-balance)
- [Testnet demo](#testnet-demo)
- [Limitations](#limitations)
- [Next steps](#next-steps)
- [Verifying the build and tests](#verifying-the-build-and-tests)

## Design overview

The prototype introduces a **pool address**: a contract that acts as the
recipient of one or more Drips streams. Funds streamed to this address accrue in
the pool. The pool tracks a balance that is only spendable by the pool admin to
open bounties.

Key properties:

- **Non-custodial streaming.** The pool does not pull funds; it is simply the
  Drips recipient. Funders keep control of their stream and can stop or adjust
  it at any time using standard Drips tooling.
- **Admin-gated spending.** Only the configured admin can open bounties from the
  pool balance. The admin cannot withdraw pool funds to an arbitrary address;
  funds can only leave the pool by opening a bounty.
- **Bounty accounting.** Each opened bounty records its amount and recipient so
  the pool balance and outstanding bounties can be reconciled.

The relevant contract sources live under `contracts/`. The pool contract is the
entry point for both receiving Drips funds and opening bounties.

## How Drips streams feed the pool

1. **Deploy the pool** with an admin address. The deployment records the admin
   and initializes an empty pool balance.
2. **Point a Drips stream at the pool address.** Using the Drips app or the
   Drips SDK, a funder creates a stream whose recipient is the pool address.
   Any number of funders can stream to the same pool.
3. **Funds accrue in the pool.** As the stream runs, the streamed amount becomes
   available to the pool. The pool balance reflects the total streamed funds
   that have not yet been committed to bounties.

Because the pool is an ordinary Drips recipient, no custom integration is
required on the funder side: existing Drips streams can be redirected to the
pool address.

## Opening bounties from the pool balance

The pool admin opens bounties against the pool balance:

1. The admin calls the pool's open-bounty entry point with the bounty amount and
   recipient details.
2. The pool checks that the requested amount does not exceed the available pool
   balance.
3. On success, the pool records the bounty and reduces the available balance
   accordingly.

Only the admin can perform this action. Non-admin callers are rejected. The
admin cannot move pool funds anywhere other than into a bounty, which keeps the
pool's purpose constrained to security bounties.

## Testnet demo

The testnet demo exercises the full flow end to end:

1. Deploy the pool contract to testnet with a demo admin account.
2. Create a Drips stream from a demo funder account to the pool address.
3. Let the stream accrue, then confirm the pool balance reflects the streamed
   funds.
4. As the admin, open a bounty from the pool balance and confirm the balance is
   reduced by the bounty amount.

Transaction links for each step are recorded in the pull request that
introduces this document (see the PR description for the `Closes #149`
reference). The links are kept in the PR rather than in this file so they stay
alongside the review discussion and can be updated as the demo is re-run.

## Limitations

- **Prototype scope.** The pool and bounty flow are a prototype. They have not
  been audited and should not hold real funds on mainnet.
- **Single admin.** Spending is gated by a single admin address. There is no
  multi-signature or role-based control in the prototype.
- **No partial-stream routing.** A stream is either directed at the pool or not;
  the prototype does not split a single stream between the pool and another
  recipient. Funders who want to route only part of their funding to bounties
  must run a separate stream to the pool.
- **No automated bounty lifecycle.** Opening a bounty is manual and admin-driven.
  Payout, review, and dispute handling are out of scope for this prototype.
- **Balance accounting is coarse.** The pool tracks a single aggregate balance;
  it does not attribute funds to individual funders or streams.
- **Testnet only.** The demo runs on testnet; no mainnet deployment is included.

## Next steps

- **Multi-signature admin.** Replace the single admin with a multisig or
  role-based access control so bounty spending requires multiple approvals.
- **Per-funder accounting.** Track which streams contributed which funds so
  funders can see how their contributions are used.
- **Automated bounty lifecycle.** Integrate bounty review, payout, and dispute
  handling so opened bounties can be settled without manual steps.
- **Stream splitting.** Support routing a fraction of a stream to the pool so
  funders do not need a separate stream.
- **Audit and mainnet plan.** Commission a review of the pool contract and
  define the path to a mainnet deployment.

## Verifying the build and tests

The acceptance criteria require the workspace to build and pass checks. From the
repository root:

```sh
cargo fmt
cargo clippy -- -D warnings
cargo test --workspace
```

To build the contract wasm:

```sh
cargo build --target wasm32-unknown-unknown --release
```

Contract test snapshots should be updated when contract behavior changes; run
the workspace tests after updating snapshots to confirm they pass.