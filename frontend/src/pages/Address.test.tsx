import { render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Bounty, Claim } from "../lib/types";
import { Address } from "./Address";

const ME = "GCFFP6RU6C3JOZBIUNZN2ELJAITUZ3HWEOUUCFXIAHE6M2GHVHZB4D6A";
const OTHER = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

const bounties: Bounty[] = [
  {
    id: 1,
    creator: ME,
    amount: 50_000_000n,
    token: "CNATIVE",
    status: "Open",
    createdAt: 1_700_000_000,
    deadline: 1_800_000_000,
    issueRef: "npm:express#typosquat",
    claimCount: 1,
    pendingClaims: 1,
  } as Bounty,
  {
    id: 2,
    creator: OTHER,
    amount: 10_000_000n,
    token: "CNATIVE",
    status: "Released",
    createdAt: 1_700_000_100,
    deadline: 1_700_500_000,
    issueRef: "npm:lodash#vulnerability",
    claimCount: 1,
    pendingClaims: 0,
  } as Bounty,
];

const claims: Record<number, Claim[]> = {
  1: [],
  2: [{ id: 0, bountyId: 2, claimant: ME, status: "Accepted", submittedAt: 1_700_000_200, proofRef: "x" } as Claim],
};

vi.mock("../lib/contract", () => ({
  listBounties: vi.fn(async () => bounties),
  listClaims: vi.fn(async (b: Bounty) => claims[b.id] ?? []),
}));

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/address/:id" element={<Address />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("Address page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("lists bounties funded and claims made by the address", async () => {
    renderAt(`/address/${ME}`);

    const funded = await screen.findByRole("heading", { name: "Bounties funded" });
    const fundedSection = funded.parentElement as HTMLElement;
    expect(within(fundedSection).getByRole("link", { name: "Bounty #1" })).toHaveAttribute("href", "/bounty/1");
    expect(within(fundedSection).queryByText("Bounty #2")).not.toBeInTheDocument();

    const claimed = screen.getByRole("heading", { name: "Claims made" });
    const claimedSection = claimed.parentElement as HTMLElement;
    expect(within(claimedSection).getByRole("link", { name: "Bounty #2" })).toHaveAttribute("href", "/bounty/2");
    expect(within(claimedSection).getByText(/claim #0/)).toBeInTheDocument();
  });

  it("shows empty states for an address with no activity", async () => {
    renderAt(`/address/${OTHER}`);
    await screen.findByText("No claims submitted by this address.");
    expect(screen.getByText("No bounties funded by this address.")).toBeInTheDocument();
  });

  it("rejects non-account addresses without querying the contract", () => {
    renderAt("/address/CNATIVE");
    expect(screen.getByText("Not a Stellar account address.")).toBeInTheDocument();
  });
});
