import { useEffect, useState } from "react";

/** Current time in unix seconds, refreshed every `intervalMs` (default one minute). */
export function useNow(intervalMs = 60_000): number {
  const [now, setNow] = useState(() => Date.now() / 1000);
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now() / 1000), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}
