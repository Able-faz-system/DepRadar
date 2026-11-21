import { describe, expect, it } from "vitest";
import { NATIVE_XLM_SAC } from "../config";
import {
  formatAmount,
  formatDuration,
  formatIssueRef,
  parseIssueRef,
  reclaimAt,
  refToUrl,
  relativeTime,
  REVIEW_WINDOW_SECS,
  shortAddress,
  timeLeft,
  tokenSymbol,
  type IssueRef,
} from "./format";

describe("formatAmount", () => {
  it("formats whole stroop amounts as XLM", () => {
    expect(formatAmount(50_000_000n)).toBe("5");
  });
  it("keeps significant decimals and trims trailing zeros", () => {
    expect(formatAmount(52_500_000n)).toBe("5.25");
    expect(formatAmount(1n)).toBe("0.0000001");
  });
  it("groups thousands", () => {
    expect(formatAmount(12_345_670_000_000n)).toBe("1,234,567");
  });
  it("handles zero and negatives", () => {
    expect(formatAmount(0n)).toBe("0");
    expect(formatAmount(-25_000_000n)).toBe("-2.5");
  });
});

describe("shortAddress", () => {
  it("shortens long Stellar addresses", () => {
    expect(shortAddress("GCFFP6RU6C3JOZBIUNZN2ELJAITUZ3HWEOUUCFXIAHE6M2GHVHZB4D6A")).toBe("GCFFP6…4D6A");
  });
  it("leaves short strings alone", () => {
    expect(shortAddress("GABC")).toBe("GABC");
  });
});

describe("tokenSymbol", () => {
  it("labels the native XLM asset contract", () => {
    expect(tokenSymbol(NATIVE_XLM_SAC)).toBe("XLM");
  });
  it("falls back to a short contract ID for other tokens", () => {
    expect(tokenSymbol("CABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVW")).toBe("CABCDE…TUVW");
  });
});

describe("relativeTime", () => {
  const now = 1_000_000;
  it("describes future deadlines", () => {
    expect(relativeTime(now + 3 * 86_400, now)).toBe("in 3 days");
  });
  it("describes past times", () => {
    expect(relativeTime(now - 2 * 3_600, now)).toBe("2 hours ago");
  });
});

describe("issue refs", () => {
  const cases: IssueRef[] = [
    { ecosystem: "npm", package: "expresss", version: "1.0.0", kind: "typosquat", detail: "express" },
    { ecosystem: "npm", package: "@babel/core", version: "7.0.0", kind: "deprecated", detail: null },
    { ecosystem: "npm", package: "@babel/core", version: null, kind: "typosquat", detail: null },
    { ecosystem: "crates.io", package: "serde", version: "1.0.0", kind: "license", detail: "MIT OR Apache-2.0" },
    {
      ecosystem: "maven",
      package: "org.apache.logging.log4j:log4j-core",
      version: "2.14.1",
      kind: "vulnerability",
      detail: "GHSA-jfh8-c2jp-5v3q",
    },
  ];

  it("formats the documented example", () => {
    expect(formatIssueRef(cases[0])).toBe("npm:expresss@1.0.0#typosquat:express");
  });
  it("round-trips", () => {
    for (const ref of cases) expect(parseIssueRef(formatIssueRef(ref))).toEqual(ref);
  });
  it("parses the ecosystem case-insensitively", () => {
    expect(parseIssueRef("PyPI:flask#deprecated")?.ecosystem).toBe("pypi");
  });
  it("rejects free-form and malformed refs", () => {
    for (const ref of [
      "npm:expresss typosquat of express (demo)",
      "org/repo#123",
      "cargo:serde#typosquat",
      "npm:#typosquat",
      "npm:lodash@#vulnerability",
      "npm:lodash@4.17.0#vulnerability:",
      "npm:lodash@4.17.0#unknown",
    ]) {
      expect(parseIssueRef(ref)).toBeNull();
    }
  });
});

describe("refToUrl", () => {
  it("returns https URLs as-is", () => {
    expect(refToUrl("https://example.com/proof")).toBe("https://example.com/proof");
    expect(refToUrl("https://github.com/DepRadarOrg/DepRadar/issues/12")).toBe(
      "https://github.com/DepRadarOrg/DepRadar/issues/12",
    );
  });
  it("converts owner/repo#N GitHub refs to issue URLs", () => {
    expect(refToUrl("DepRadarOrg/DepRadar#12")).toBe("https://github.com/DepRadarOrg/DepRadar/issues/12");
    expect(refToUrl("owner/repo#1")).toBe("https://github.com/owner/repo/issues/1");
  });
  it("leaves plain text and standard issue refs as text", () => {
    expect(refToUrl("see the attached screenshot")).toBeNull();
    expect(refToUrl("npm:expresss@1.0.0#typosquat:express")).toBeNull();
    expect(refToUrl("")).toBeNull();
  });
  it("never turns non-https schemes into links", () => {
    expect(refToUrl("javascript:alert(1)")).toBeNull();
    expect(refToUrl("JavaScript:alert(1)")).toBeNull();
    expect(refToUrl("data:text/html,<script>alert(1)</script>")).toBeNull();
    expect(refToUrl("http://example.com/proof")).toBeNull();
    expect(refToUrl("ftp://example.com/file")).toBeNull();
  });
});

describe("formatDuration", () => {
  it("uses the two largest units", () => {
    expect(formatDuration(3 * 86_400 + 4 * 3_600 + 59)).toBe("3d 4h");
    expect(formatDuration(4 * 3_600 + 12 * 60)).toBe("4h 12m");
    expect(formatDuration(12 * 60)).toBe("12m");
  });
  it("shows <1m under a minute and clamps negatives", () => {
    expect(formatDuration(59)).toBe("<1m");
    expect(formatDuration(-5)).toBe("<1m");
  });
});

describe("timeLeft", () => {
  const deadline = 1_000_000;
  it("counts down while the deadline is ahead", () => {
    expect(timeLeft(deadline, deadline - 3 * 86_400 - 4 * 3_600)).toBe("3d 4h left");
    expect(timeLeft(deadline, deadline - 1)).toBe("<1m left");
  });
  it("is null exactly at the deadline and after", () => {
    expect(timeLeft(deadline, deadline)).toBeNull();
    expect(timeLeft(deadline, deadline + 1)).toBeNull();
  });
});

describe("review window", () => {
  const deadline = 1_000_000;
  it("is seven days after the deadline", () => {
    expect(REVIEW_WINDOW_SECS).toBe(7 * 86_400);
    expect(reclaimAt(deadline)).toBe(deadline + 7 * 86_400);
  });
  it("switches to reclaimable exactly at the boundary", () => {
    const reclaim = reclaimAt(deadline);
    expect(timeLeft(reclaim, reclaim - 1)).toBe("<1m left");
    expect(timeLeft(reclaim, reclaim)).toBeNull();
  });
});
