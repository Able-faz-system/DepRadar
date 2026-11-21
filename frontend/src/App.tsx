import { lazy, Suspense } from "react";
import { HashRouter, Route, Routes } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { Layout } from "./components/Layout";
import { Empty, Loading } from "./components/ui";

// Pages are lazy so the shell renders immediately and the Stellar SDK (pulled
// in by the contract layer) loads only with the routes that need it.
const About = lazy(() => import("./pages/About").then((m) => ({ default: m.About })));
const BountyDetail = lazy(() => import("./pages/BountyDetail").then((m) => ({ default: m.BountyDetail })));
const BountyList = lazy(() => import("./pages/BountyList").then((m) => ({ default: m.BountyList })));
const Leaderboard = lazy(() => import("./pages/Leaderboard").then((m) => ({ default: m.Leaderboard })));
const Report = lazy(() => import("./pages/Report").then((m) => ({ default: m.Report })));

/** Hash routing so the build works on any static host with no rewrite rules. */
export default function App() {
  const { t } = useTranslation();
  return (
    <ErrorBoundary>
      <HashRouter>
        <Suspense fallback={<Loading />}>
          <Routes>
            <Route element={<Layout />}>
              <Route index element={<BountyList />} />
              <Route path="bounty/:id" element={<BountyDetail />} />
              <Route path="leaderboard" element={<Leaderboard />} />
              <Route path="report" element={<Report />} />
              <Route path="about" element={<About />} />
              <Route path="*" element={<Empty>{t("app.notFound")}</Empty>} />
            </Route>
          </Routes>
        </Suspense>
      </HashRouter>
    </ErrorBoundary>
  );
}
