import { NATIVE_XLM_SAC } from "../config";
import type { Bounty, BountyStatus, Claim, ClaimStatus } from "./types";

export interface BoardStats {
  total: number;
  open: number;
  paid: number;
  refunded: number;
  /** Native XLM currently locked in open bounties (stroops). */
  lockedXlm: bigint;
  /** Native XLM paid out to researchers (stroops). */
  paidXlm: bigint;
  claims: number;
}

export function boardStats(bounties: Bounty[]): BoardStats {
  const stats: BoardStats = { total: bounties.length, open: 0, paid: 0, refunded: 0, lockedXlm: 0n, paidXlm: 0n, claims: 0 };
  for (const b of bounties) {
    stats.claims += b.claimCount;
    const xlm = b.token === NATIVE_XLM_SAC ? b.amount : 0n;
    if (b.status === "Open") {
      stats.open += 1;
      stats.lockedXlm += xlm;
    } else if (b.status === "Released") {
      stats.paid += 1;
      stats.paidXlm += xlm;
    } else {
      stats.refunded += 1;
    }
  }
  return stats;
}

/** User-facing names for contract states. */
export const BOUNTY_LABEL: Record<BountyStatus, string> = {
  Open: "Open",
  Released: "Paid",
  Cancelled: "Refunded",
};

export const CLAIM_LABEL: Record<ClaimStatus, string> = {
  Pending: "Under review",
  Accepted: "Accepted",
  Rejected: "Rejected",
};

export type TimelineKind = "created" | "claim" | "deadline" | "paid" | "refunded";

export interface TimelineEvent {
  kind: TimelineKind;
  /** Unix seconds, when known. Payout/refund times aren't stored on-chain. */
  at?: number;
  title: string;
  claim?: Claim;
  /** Deadline that hasn't happened yet. */
  upcoming?: boolean;
}

/**
 * The bounty's history in order: creation, every claim, the deadline, and
 * the final outcome. The contract doesn't store when a bounty was paid or
 * refunded, so that step comes last without a timestamp.
 */
export function bountyTimeline(bounty: Bounty, claims: Claim[], now = Date.now() / 1000): TimelineEvent[] {
  const events: TimelineEvent[] = [{ kind: "created", at: bounty.createdAt, title: "Bounty funded" }];

  for (const claim of claims) {
    events.push({ kind: "claim", at: claim.submittedAt, title: `Claim #${claim.id} submitted`, claim });
  }

  const deadlinePassed = bounty.deadline <= now;
  if (bounty.status === "Open" || deadlinePassed) {
    events.push({
      kind: "deadline",
      at: bounty.deadline,
      title: deadlinePassed ? "Claims closed" : "Claims close",
      upcoming: !deadlinePassed,
    });
  }

  events.sort((a, b) => (a.at ?? Infinity) - (b.at ?? Infinity));

  const winner = claims.find((c) => c.status === "Accepted");
  if (bounty.status === "Released") {
    events.push({ kind: "paid", title: winner ? `Paid to claim #${winner.id}` : "Paid out", claim: winner });
  } else if (bounty.status === "Cancelled") {
    events.push({ kind: "refunded", title: "Refunded to creator" });
  }
  return events;
}

export interface ResearcherRank {
  claimant: string;
  totalPaid: bigint;
  count: number;
}

/**
 * Ranks researchers by total earnings from released bounties.
 * Order: totalPaid descending, count descending, claimant address ascending.
 */
export function rankResearchers(bounties: Bounty[], claims: Claim[]): ResearcherRank[] {
  const released = new Map<number, Bounty>();
  for (const b of bounties) {
    if (b.status === "Released") {
      released.set(b.id, b);
    }
  }

  const aggregates = new Map<string, { totalPaid: bigint; count: number }>();

  for (const c of claims) {
    if (c.status === "Accepted") {
      const bounty = released.get(c.bountyId);
      if (bounty) {
        const entry = aggregates.get(c.claimant) ?? { totalPaid: 0n, count: 0 };
        entry.totalPaid += bounty.amount;
        entry.count += 1;
        aggregates.set(c.claimant, entry);
      }
    }
  }

  const rankings: ResearcherRank[] = [];
  for (const [claimant, { totalPaid, count }] of aggregates.entries()) {
    rankings.push({ claimant, totalPaid, count });
  }

  rankings.sort((a, b) => {
    if (b.totalPaid !== a.totalPaid) {
      return b.totalPaid > a.totalPaid ? 1 : -1;
    }
    if (b.count !== a.count) {
      return b.count - a.count;
    }
    return a.claimant.localeCompare(b.claimant);
  });

  return rankings;
}
