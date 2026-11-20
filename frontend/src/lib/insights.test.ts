import { describe, expect, it } from "vitest";
import { NATIVE_XLM_SAC } from "../config";
import { boardStats, bountyTimeline, rankResearchers } from "./insights";
import type { Bounty, Claim } from "./types";

const base: Bounty = {
  id: 1,
  creator: "GCREATOR",
  issueRef: "npm:expresss",
  amount: 50_000_000n,
  token: NATIVE_XLM_SAC,
  status: "Open",
  createdAt: 1_000,
  deadline: 5_000,
  claimCount: 0,
  pendingClaims: 0,
};

const claim = (id: number, at: number, status: Claim["status"]): Claim => ({
  bountyId: 1,
  id,
  claimant: `G${id}`,
  proofRef: "pr",
  submittedAt: at,
  status,
});

describe("boardStats", () => {
  it("splits XLM between locked and paid and counts outcomes", () => {
    const stats = boardStats([
      { ...base, id: 1, status: "Open", claimCount: 1 },
      { ...base, id: 2, status: "Released", amount: 20_000_000n, claimCount: 2 },
      { ...base, id: 3, status: "Cancelled" },
    ]);
    expect(stats).toEqual({
      total: 3,
      open: 1,
      paid: 1,
      refunded: 1,
      lockedXlm: 50_000_000n,
      paidXlm: 20_000_000n,
      claims: 3,
    });
  });

  it("ignores non-XLM tokens in the XLM totals", () => {
    const stats = boardStats([{ ...base, token: "COTHER" }]);
    expect(stats.lockedXlm).toBe(0n);
    expect(stats.open).toBe(1);
  });
});

describe("bountyTimeline", () => {
  it("orders creation, claims and an upcoming deadline for an open bounty", () => {
    const events = bountyTimeline(base, [claim(1, 3_000, "Pending"), claim(0, 2_000, "Rejected")], 4_000);
    expect(events.map((e) => e.title)).toEqual([
      "Bounty funded",
      "Claim #0 submitted",
      "Claim #1 submitted",
      "Claims close",
    ]);
    expect(events.at(-1)?.upcoming).toBe(true);
  });

  it("ends a paid bounty with the winning claim", () => {
    const events = bountyTimeline(
      { ...base, status: "Released" },
      [claim(0, 2_000, "Rejected"), claim(1, 3_000, "Accepted")],
      4_000,
    );
    expect(events.at(-1)).toMatchObject({ kind: "paid", title: "Paid to claim #1" });
    expect(events.some((e) => e.kind === "deadline")).toBe(false);
  });

  it("shows the passed deadline before a refund", () => {
    const events = bountyTimeline({ ...base, status: "Cancelled" }, [], 9_000);
    expect(events.map((e) => e.kind)).toEqual(["created", "deadline", "refunded"]);
    expect(events[1].title).toBe("Claims closed");
  });
});

describe("rankResearchers", () => {
  it("ranks researchers by total payout descending", () => {
    const bounties: Bounty[] = [
      { ...base, id: 1, status: "Released", amount: 100_000_000n },
      { ...base, id: 2, status: "Released", amount: 40_000_000n },
      { ...base, id: 3, status: "Released", amount: 30_000_000n },
    ];
    const claims: Claim[] = [
      { ...claim(1, 1_000, "Accepted"), bountyId: 1, claimant: "ALICE" },
      { ...claim(2, 2_000, "Accepted"), bountyId: 2, claimant: "BOB" },
      { ...claim(3, 3_000, "Accepted"), bountyId: 3, claimant: "BOB" },
    ];
    const ranked = rankResearchers(bounties, claims);
    expect(ranked).toEqual([
      { claimant: "ALICE", totalPaid: 100_000_000n, count: 1 },
      { claimant: "BOB", totalPaid: 70_000_000n, count: 2 },
    ]);
  });

  it("breaks payout ties by count descending, then address ascending", () => {
    const bounties: Bounty[] = [
      { ...base, id: 1, status: "Released", amount: 50_000_000n },
      { ...base, id: 2, status: "Released", amount: 25_000_000n },
      { ...base, id: 3, status: "Released", amount: 25_000_000n },
      { ...base, id: 4, status: "Released", amount: 50_000_000n },
    ];
    const claims: Claim[] = [
      { ...claim(1, 1_000, "Accepted"), bountyId: 1, claimant: "CHARLIE" },
      { ...claim(2, 2_000, "Accepted"), bountyId: 2, claimant: "BOB" },
      { ...claim(3, 3_000, "Accepted"), bountyId: 3, claimant: "BOB" },
      { ...claim(4, 4_000, "Accepted"), bountyId: 4, claimant: "ALICE" },
    ];
    const ranked = rankResearchers(bounties, claims);
    expect(ranked).toEqual([
      { claimant: "BOB", totalPaid: 50_000_000n, count: 2 },
      { claimant: "ALICE", totalPaid: 50_000_000n, count: 1 },
      { claimant: "CHARLIE", totalPaid: 50_000_000n, count: 1 },
    ]);
  });

  it("ignores unreleased bounties and unaccepted claims", () => {
    const bounties: Bounty[] = [
      { ...base, id: 1, status: "Open", amount: 100_000_000n },
      { ...base, id: 2, status: "Cancelled", amount: 100_000_000n },
      { ...base, id: 3, status: "Released", amount: 50_000_000n },
    ];
    const claims: Claim[] = [
      { ...claim(1, 1_000, "Accepted"), bountyId: 1, claimant: "ALICE" },
      { ...claim(2, 2_000, "Accepted"), bountyId: 2, claimant: "BOB" },
      { ...claim(3, 3_000, "Pending"), bountyId: 3, claimant: "CHARLIE" },
      { ...claim(4, 4_000, "Rejected"), bountyId: 3, claimant: "DAVE" },
      { ...claim(5, 5_000, "Accepted"), bountyId: 3, claimant: "EVE" },
    ];
    const ranked = rankResearchers(bounties, claims);
    expect(ranked).toEqual([
      { claimant: "EVE", totalPaid: 50_000_000n, count: 1 },
    ]);
  });
});
