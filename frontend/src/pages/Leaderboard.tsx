import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { AddressLink, Empty, ErrorState, Skeleton } from "../components/ui";
import { listBounties, listClaims } from "../lib/contract";
import { formatAmount } from "../lib/format";
import { rankResearchers, type ResearcherRank } from "../lib/insights";
import { useAsync } from "../lib/useAsync";

export function Leaderboard() {
  const { t } = useTranslation();

  const state = useAsync(async (): Promise<ResearcherRank[]> => {
    const bounties = await listBounties();
    const released = bounties.filter((b) => b.status === "Released" && b.claimCount > 0);
    const claimsArrays = await Promise.all(released.map((b) => listClaims(b)));
    const allClaims = claimsArrays.flat();
    return rankResearchers(bounties, allClaims);
  }, "leaderboard");

  return (
    <div className="page leaderboard-page">
      <div style={{ marginBottom: "2rem" }}>
        <Link to="/" className="back" style={{ display: "inline-block", marginBottom: "1rem" }}>
          {t("app.backToBoard")}
        </Link>
        <h1 style={{ fontSize: "2.2rem", fontWeight: 700, marginBottom: "0.5rem" }}>
          {t("leaderboard.title")}
        </h1>
        <p className="muted" style={{ fontSize: "1.05rem" }}>
          {t("leaderboard.subtitle")}
        </p>
      </div>

      {state.status === "loading" && <LeaderboardSkeleton />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={state.reload} />}
      {state.status === "success" &&
        (state.data.length === 0 ? (
          <Empty>
            <p>{t("leaderboard.empty")}</p>
          </Empty>
        ) : (
          <div className="card" style={{ padding: "1.5rem", overflowX: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", textAlign: "left" }}>
              <thead>
                <tr
                  style={{
                    borderBottom: "1px solid var(--border)",
                    color: "var(--muted)",
                    fontSize: "0.85rem",
                    textTransform: "uppercase",
                  }}
                >
                  <th style={{ padding: "0.75rem 1rem", width: "4rem" }}>#</th>
                  <th style={{ padding: "0.75rem 1rem" }}>{t("leaderboard.researcher")}</th>
                  <th style={{ padding: "0.75rem 1rem", textAlign: "right" }}>
                    {t("leaderboard.bountiesSolved")}
                  </th>
                  <th style={{ padding: "0.75rem 1rem", textAlign: "right" }}>
                    {t("leaderboard.totalEarned")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {state.data.map((rank, index) => (
                  <tr
                    key={rank.claimant}
                    style={{
                      borderBottom:
                        index < state.data.length - 1
                          ? "1px solid var(--border-subtle, rgba(255, 255, 255, 0.05))"
                          : "none",
                    }}
                  >
                    <td
                      style={{
                        padding: "1rem",
                        fontWeight: 600,
                        color: index < 3 ? "var(--teal)" : "var(--muted)",
                      }}
                    >
                      {index === 0 ? "🥇 1" : index === 1 ? "🥈 2" : index === 2 ? "🥉 3" : `${index + 1}`}
                    </td>
                    <td style={{ padding: "1rem" }}>
                      <AddressLink address={rank.claimant} avatar />
                    </td>
                    <td
                      style={{
                        padding: "1rem",
                        textAlign: "right",
                        fontFamily: "var(--font-mono, monospace)",
                      }}
                    >
                      {rank.count}
                    </td>
                    <td style={{ padding: "1rem", textAlign: "right", fontWeight: 600 }}>
                      <span style={{ color: "var(--green, #22c55e)" }}>
                        {formatAmount(rank.totalPaid)}
                      </span>
                      <span className="muted" style={{ marginLeft: "0.35rem", fontSize: "0.85rem" }}>
                        XLM
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))}
    </div>
  );
}

function LeaderboardSkeleton() {
  return (
    <div className="card" style={{ padding: "1.5rem" }} aria-hidden>
      <div style={{ display: "grid", gap: "1rem" }}>
        {[0, 1, 2, 3, 4].map((i) => (
          <div
            key={i}
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              padding: "0.5rem 0",
            }}
          >
            <Skeleton className="sk-sm" />
            <Skeleton className="sk-title" />
            <Skeleton className="sk-sm" />
            <Skeleton className="sk-stat" />
          </div>
        ))}
      </div>
    </div>
  );
}

export default Leaderboard;
