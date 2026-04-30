import { useCallback, useEffect, useState } from "react";
import { getStats } from "@/lib/tauri";
import type { StatsResult } from "@/lib/types";

export function useStats(intervalMs = 10000) {
  const [stats, setStats] = useState<StatsResult | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await getStats();
      setStats(s);
    } catch {
      setStats(null);
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, intervalMs);
    return () => clearInterval(id);
  }, [refresh, intervalMs]);

  return { stats, refresh };
}
