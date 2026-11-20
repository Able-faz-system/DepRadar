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
 */
import { pathToFileURL } from "node:url";
import { rpc } from "@stellar/stellar-sdk";

export const DEFAULT_CONTRACT_ID = "CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ";
export const DEFAULT_RPC_URL = "https://soroban-testnet.stellar.org";

export interface ParsedArgs {
  bountyId: number;
  contractId: string;
  rpcUrl: string;
}

export type Env = Record<string, string | undefined>;

export function parseArgs(argv: string[], env: Env = process.env): ParsedArgs {
  const flags = new Map<string, string>();
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg.startsWith("--")) {
      flags.set(arg.slice(2), argv[i + 1]);
      i += 1;
    }
  }

  const rawBountyId = flags.get("bounty-id") ?? "1";
  const bountyId = Number(rawBountyId);
  if (!Number.isInteger(bountyId) || bountyId < 0) {
    throw new Error(`--bounty-id must be a non-negative integer, got: ${rawBountyId}`);
  }

  const contractId = flags.get("contract") ?? env.DEPRADAR_CONTRACT_ID ?? DEFAULT_CONTRACT_ID;
  const rpcUrl = flags.get("rpc") ?? env.DEPRADAR_RPC_URL ?? DEFAULT_RPC_URL;

  return { bountyId, contractId, rpcUrl };
}

export function bigIntReplacer(_key: string, value: unknown) {
  return typeof value === "bigint" ? value.toString() : value;
}

export interface FormatterInput {
  bountyId: number;
  contractId: string;
  rpcUrl: string;
  health: { status: string; latestLedger: number };
  bountyCount: unknown;
  bounty: unknown;
  claims: unknown[];
}

/** Renders the full read-client report as a list of output lines. */
export function formatOutput(input: FormatterInput): string[] {
  const { bountyId, contractId, rpcUrl, health, bountyCount, bounty, claims } = input;
  const lines: string[] = [];

  lines.push(`Connected to ${rpcUrl} (status: ${health.status}, latest ledger: ${health.latestLedger})`);
  lines.push(`Contract: ${contractId}`);
  lines.push(`Bounties created: ${bountyCount}\n`);

  lines.push(`get_bounty(${bountyId}) =>`);
  if (bounty === null) {
    lines.push("  (no bounty with this ID)");
    return lines;
  }
  lines.push(JSON.stringify(bounty, bigIntReplacer, 2));

  claims.forEach((claim, claimId) => {
    lines.push(`\nget_claim(${bountyId}, ${claimId}) =>`);
    lines.push(claim === null ? "  (no claim with this ID)" : JSON.stringify(claim, bigIntReplacer, 2));
  });

  return lines;
}

async function main() {
  const { bountyId, contractId, rpcUrl } = parseArgs(process.argv.slice(2));
  const server = new rpc.Server(rpcUrl);

  const health = await server.getHealth();
  const { result: bountyCount } = await server.queryContract(contractId, "bounty_count", {});
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

  const lines = formatOutput({
    bountyId,
    contractId,
    rpcUrl,
    health: { status: health.status, latestLedger: health.latestLedger },
    bountyCount,
    bounty,
    claims,
  });
  for (const line of lines) {
    console.log(line);
  }
}

const isDirectRun =
  process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;

if (isDirectRun) {
  main().catch((err) => {
    console.error("Read failed:", err instanceof Error ? err.message : err);
    process.exitCode = 1;
  });
}
