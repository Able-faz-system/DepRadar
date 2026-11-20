/**
 * Minimal read-only client for the deprader_router Soroban contract on
 * Stellar testnet. Fetches a bounty and every claim submitted against it
 * and prints the result — no submitted transactions, no UI.
 *
 * Usage:
 *   npm start -- --bounty-id 1
 *   npm start -- --bounty-id 1 --contract C... --rpc https://...
 *
 * --bounty-id defaults to 1, the bounty used in the on-chain
 * create_bounty -> submit_claim -> reject_claim -> release cycle described
 * in PROGRESS.md, so running with no flags exercises real data.
 *
 * --contract / --rpc override the target deployment. When omitted, the
 * DEPRADAR_CONTRACT_ID / DEPRADAR_RPC_URL env vars are used, falling back
 * to the default testnet deployment below.
 *
 * --all fetches every bounty (using bounty_count) and its claims instead
 * of a single bounty. --json prints a single JSON document instead of the
 * human-readable output; combined with --all it includes every bounty.
 */
import { rpc } from "@stellar/stellar-sdk";
import { parseBounty, parseClaim } from "@deprader/contract-types";

const DEFAULT_CONTRACT_ID = "CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ";
const DEFAULT_RPC_URL = "https://soroban-testnet.stellar.org";

const BOOLEAN_FLAGS = new Set(["all", "json"]);

interface Options {
  bountyId: number;
  contractId: string;
  rpcUrl: string;
  all: boolean;
  json: boolean;
}

function parseArgs(argv: string[]): Options {
  const flags = new Map<string, string>();
  const booleans = new Set<string>();
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg.startsWith("--")) {
      const name = arg.slice(2);
      if (BOOLEAN_FLAGS.has(name)) {
        booleans.add(name);
      } else {
        flags.set(name, argv[i + 1]);
        i += 1;
      }
    }
  }

  const bountyId = Number(flags.get("bounty-id") ?? "1");
  if (!Number.isInteger(bountyId) || bountyId < 0) {
    throw new Error(`--bounty-id must be a non-negative integer, got: ${flags.get("bounty-id")}`);
  }

  const contractId = flags.get("contract") ?? process.env.DEPRADAR_CONTRACT_ID ?? DEFAULT_CONTRACT_ID;
  const rpcUrl = flags.get("rpc") ?? process.env.DEPRADAR_RPC_URL ?? DEFAULT_RPC_URL;

  return { bountyId, contractId, rpcUrl, all: booleans.has("all"), json: booleans.has("json") };
}

interface BountyResult {
  bounty_id: number;
  bounty: unknown;
  claims: unknown[];
}

async function fetchBounty(server: rpc.Server, contractId: string, bountyId: number): Promise<BountyResult> {
  const { result: bounty } = await server.queryContract(contractId, "get_bounty", { bounty_id: bountyId });
  const claims: unknown[] = [];
  if (bounty !== null) {
    const claimCount = Number((bounty as { claim_count: number }).claim_count);
    for (let claimId = 0; claimId < claimCount; claimId += 1) {
      const { result: claim } = await server.queryContract(contractId, "get_claim", {
        bounty_id: bountyId,
        claim_id: claimId,
      });
      claims.push(claim);
    }
  }
  return { bounty_id: bountyId, bounty, claims };
}

function printBounty(result: BountyResult) {
  const { bounty_id: bountyId, bounty: rawBounty, claims: rawClaims } = result;
  console.log(`get_bounty(${bountyId}) =>`);
  if (rawBounty === null) {
    console.log("  (no bounty with this ID)");
    return;
  }
  const bounty = parseBounty(rawBounty);
  console.log(JSON.stringify(bounty, bigIntReplacer, 2));

  rawClaims.forEach((rawClaim, claimId) => {
    console.log(`\nget_claim(${bountyId}, ${claimId}) =>`);
    if (rawClaim === null) {
      console.log("  (no claim with this ID)");
      return;
    }
    const claim = parseClaim(rawClaim);
    console.log(JSON.stringify(claim, bigIntReplacer, 2));
  });
}

async function main() {
  const { bountyId, contractId, rpcUrl, all, json } = parseArgs(process.argv.slice(2));
  const server = new rpc.Server(rpcUrl);

  const health = await server.getHealth();
  const { result: count } = await server.queryContract(contractId, "bounty_count", {});

  if (!json) {
    console.log(`Connected to ${rpcUrl} (status: ${health.status}, latest ledger: ${health.latestLedger})`);
    console.log(`Contract: ${contractId}`);
    console.log(`Bounties created: ${count}\n`);
  }

  const bountyIds = all
    ? Array.from({ length: Number(count) }, (_unused, index) => index)
    : [bountyId];

  const results: BountyResult[] = [];
  for (const id of bountyIds) {
    results.push(await fetchBounty(server, contractId, id));
  }

  if (json) {
    const document = all
      ? { contract_id: contractId, bounty_count: count, bounties: results }
      : { contract_id: contractId, bounty_count: count, bounty: results[0] };
    console.log(JSON.stringify(document, bigIntReplacer, 2));
    return;
  }

  for (const result of results) {
    printBounty(result);
  }
}

function bigIntReplacer(_key: string, value: unknown) {
  return typeof value === "bigint" ? value.toString() : value;
}

main().catch((err) => {
  console.error("Read failed:", err instanceof Error ? err.message : err);
  process.exitCode = 1;
});
