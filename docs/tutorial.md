# Tutorial: Report Your First Typosquat

This tutorial walks you through the complete process: finding a typosquat with DepRadar's scanner, reporting it to the registry, and claiming a bounty on Stellar testnet.

By the end you'll have scanned a package, understood the finding, reported it, created a bounty, submitted a claim, and seen it paid out.

## Prerequisites

- **Rust toolchain** (for building from source) or a [prebuilt CLI release](https://github.com/DepRadarOrg/DepRadar/releases/latest)
- **Node.js 22+** (for the testnet client)
- **Stellar testnet account** with XLM from the [friendbot faucet](https://laboratory.stellar.org/#account-creator?network=test)
- **stellar-cli** installed: `cargo install --locked stellar-cli`

## Step 1: Scan a Package

Let's scan a known typosquat. Create a test `package-lock.json`:

```json
{
  "name": "test-project",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "dependencies": {
        "expresss": "1.0.0"
      }
    },
    "node_modules/expresss": {
      "version": "1.0.0",
      "resolved": "https://registry.npmjs.org/expresss/-/expresss-1.0.0.tgz"
    }
  }
}
```

Run the scanner:

```sh
cargo run --bin depradar -- scan package-lock.json
```

Or with a prebuilt binary:

```sh
depradar scan package-lock.json
```

**Expected output:**

```
[MEDIUM] Typosquat: expresss
  Matches popular package: express (distance: 1)
  Confidence: medium
  Ecosystem: npm

1 finding (0 critical, 0 high, 1 medium, 0 low)
```

The scanner flagged `expresss` as a typosquat of `express` (one extra `s`). The edit distance is 1 and confidence is medium.

### Understanding the Result

- **Severity**: `MEDIUM` — typosquats are rated medium by default unless they also trigger behavioral or CVE signals
- **Distance**: `1` — one character different from the popular package
- **Confidence**: derived from the distance and whether multiple signals fire (homoglyph, combosquat, etc.)

For JSON output with all details:

```sh
depradar scan package-lock.json --format json
```

## Step 2: Verify the Finding

Before reporting, confirm it's actually a typosquat:

1. **Check npm**: Visit `https://www.npmjs.com/package/expresss` — if it exists and has suspicious code, it's a real finding
2. **Check GitHub**: Look for the package's repository — many typosquats have no repo or a cloned one
3. **Review the code**: `npm view expresss` or download and inspect — does it install the real `express`, or is it malicious?

For this tutorial, we'll assume `expresss` is a deliberate typosquat (install script, credential stealer, or just confusion).

## Step 3: Report to the Registry

If the package is still live on npm, report it:

1. Go to [npm's malware report form](https://www.npmjs.com/support)
2. Select "I want to report malware"
3. Provide:
   - Package name: `expresss`
   - Reason: "Typosquat of `express` (edit distance 1, detected by DepRadar scanner)"
   - Evidence: link to your scan output, screenshot, or a GitHub gist with the lockfile and result

npm typically responds within 1–3 business days. If they unpublish it, great — the finding is confirmed. If not, document their response.

## Step 4: Fund a Bounty (Testnet)

Now let's put a bounty on this finding. We'll use Stellar testnet.

### 4a. Get Testnet XLM

Visit the [Stellar Laboratory account creator](https://laboratory.stellar.org/#account-creator?network=test) and fund an account. Save the secret key — you'll need it.

Alternatively, use `stellar-cli`:

```sh
stellar keys generate alice --network testnet
stellar keys fund alice --network testnet
```

### 4b. Get the Native XLM Token Address

The testnet native token (XLM) address is:

```
CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

### 4c. Create the Bounty

We'll use `stellar-cli` to call the contract. The testnet contract ID is:

```
CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ
```

Create a bounty for 10 XLM (100000000 stroops), with a 7-day deadline:

```sh
stellar contract invoke \
  --id CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ \
  --source alice \
  --network testnet \
  -- \
  create_bounty \
  --creator alice \
  --issue_ref "npm:expresss typosquat of express, distance=1" \
  --amount 100000000 \
  --token CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC \
  --deadline $(($(date +%s) + 604800))
```

**Output:**

```
1
```

That's your bounty ID. The contract locked 10 XLM from your account.

### 4d. Verify the Bounty

Read it back with the client:

```sh
cd client
npm ci
npm start -- --bounty-id 1
```

**Output:**

```json
{
  "id": 1,
  "creator": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  "issueRef": "npm:expresss typosquat of express, distance=1",
  "amount": "100000000",
  "token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "status": "Open",
  "claimCount": 0,
  "deadline": 1700000000,
  "createdAt": 1699395200
}
```

Or visit the [dashboard](https://depradarorg.github.io/DepRadar/) and find your bounty in the list.

## Step 5: Submit a Claim

Now act as the researcher who found it. You'll submit a claim with proof.

### 5a. Write the Proof

Your proof is a reference to where you disclosed the finding. This could be:

- A GitHub issue in the affected package's repo (if it has one)
- A gist with your scan output and npm report confirmation
- A PR to the registry's security tracker
- A blog post or advisory

For this tutorial, let's say you opened a gist at `https://gist.github.com/yourname/abc123def456` with the scan result and npm report.

### 5b. Submit the Claim

Generate a second testnet identity (the claimant):

```sh
stellar keys generate bob --network testnet
stellar keys fund bob --network testnet
```

Submit the claim:

```sh
stellar contract invoke \
  --id CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ \
  --source bob \
  --network testnet \
  -- \
  submit_claim \
  --bounty_id 1 \
  --claimant bob \
  --proof_ref "https://gist.github.com/yourname/abc123def456"
```

**Output:**

```
0
```

That's your claim ID. The contract recorded your claim (status: `Pending`).

### 5c. Verify the Claim

```sh
cd client
npm start -- --bounty-id 1 --claim-id 0
```

**Output:**

```json
{
  "bountyId": 1,
  "claimId": 0,
  "claimant": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  "proofRef": "https://gist.github.com/yourname/abc123def456",
  "status": "Pending",
  "submittedAt": 1699395300
}
```

Or check the dashboard — the bounty page now shows your pending claim.

## Step 6: Admin Review and Release

In a real scenario, the contract's admin reviews your `proof_ref`, confirms the finding is legitimate, and releases the bounty. On testnet, you can simulate this if you have the admin key (not shared publicly).

The admin would run:

```sh
stellar contract invoke \
  --id CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ \
  --source admin \
  --network testnet \
  -- \
  release \
  --bounty_id 1 \
  --claim_id 0
```

This transfers the 10 XLM from the contract to bob's account and closes the bounty (status: `PaidOut`).

### 6a. Verify the Payout

Check bob's balance:

```sh
stellar keys address bob | xargs -I {} \
  stellar contract invoke \
  --id CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC \
  --source bob \
  --network testnet \
  -- \
  balance \
  --id {}
```

It should be 10 XLM higher than before the release.

Or check the dashboard — the bounty's status is now "Paid Out" and the claim is "Accepted".

## Step 7: View on Stellar Expert

Every transaction is public. Visit [Stellar Expert testnet](https://stellar.expert/explorer/testnet) and search for the contract ID:

```
CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ
```

You'll see the `create_bounty`, `submit_claim`, and `release` calls in the contract's history.

## Step 8: Try the Dashboard

Open the [DepRadar dashboard](https://depradarorg.github.io/DepRadar/) in your browser. You'll see:

- **Home page**: all bounties, with live stats (total paid out, bounties funded, claims submitted)
- **Bounty list**: filter by Open / Paid Out / Refunded
- **Bounty detail** (click a bounty): the full timeline, all claims, and a copyable finding reference

The dashboard is read-only — it doesn't need your wallet and can't create or claim bounties. It's a pure view over the contract's state.

## What You've Learned

- How to **scan a lockfile** and interpret typosquat findings
- How to **report a malicious package** to the registry
- How to **create a bounty** on Stellar testnet with `stellar-cli`
- How to **submit a claim** with a proof reference
- How the **admin review** and **payout** process works
- How to **verify everything** on-chain with the client or Stellar Expert

## Next Steps

- Scan your own projects: `depradar scan path/to/package-lock.json`
- Try other detection types: the scanner also checks for CVEs (`--offline` to skip OSV)
- Explore the contract's [threat model](threat-model.md) and [architecture](architecture.md)
- Read the [FAQ](FAQ.md) for common questions

## Troubleshooting

### "Authorization failed" when creating bounty or submitting claim

Make sure you're using `--source <identity>` and the identity matches the `--creator` or `--claimant` argument.

### "Bounty not found"

The bounty ID is contract-assigned. Check the dashboard or `client/` to see the current `bounty_count`.

### "Token contract error"

Make sure the token address is correct for testnet native XLM:

```
CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

### "Deadline in the past"

The `--deadline` argument is a Unix timestamp in seconds. Use `$(date +%s)` for the current time and add seconds (e.g. `+ 604800` for 7 days).

### Scanner finds nothing

Make sure the package name is in the lockfile exactly as it appears on the registry (lowercase, hyphens not underscores). Scoped packages need the `@scope/name` format.

## Commands Reference

| Task | Command |
|------|---------|
| Scan a lockfile | `depradar scan <lockfile>` |
| JSON output | `depradar scan <lockfile> --format json` |
| Fail on high severity | `depradar scan <lockfile> --fail-on high` |
| Generate a testnet key | `stellar keys generate <name> --network testnet` |
| Fund a testnet account | `stellar keys fund <name> --network testnet` |
| Create a bounty | `stellar contract invoke --id <contract> --source <identity> --network testnet -- create_bounty --creator <identity> --issue_ref "<ref>" --amount <stroops> --token <token> --deadline <timestamp>` |
| Submit a claim | `stellar contract invoke --id <contract> --source <identity> --network testnet -- submit_claim --bounty_id <id> --claimant <identity> --proof_ref "<url>"` |
| Read a bounty | `cd client && npm start -- --bounty-id <id>` |
| Read a claim | `cd client && npm start -- --bounty-id <id> --claim-id <id>` |

## Resources

- [DepRadar GitHub](https://github.com/DepRadarOrg/DepRadar)
- [Testnet contract](https://stellar.expert/explorer/testnet/contract/CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ)
- [Dashboard](https://depradarorg.github.io/DepRadar/)
- [Stellar Laboratory](https://laboratory.stellar.org/)
- [Stellar CLI docs](https://developers.stellar.org/docs/tools/stellar-cli)
