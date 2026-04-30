import { useCallback, useEffect, useState } from "react";
import { getProxyStatus } from "@/lib/tauri";
import type { ProxyStatus } from "@/lib/types";

export function useProxyStatus(intervalMs = 5000) {
  const [status, setStatus] = useState<ProxyStatus | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await getProxyStatus();
      setStatus(s);
    } catch {
      setStatus(null);
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, intervalMs);
    return () => clearInterval(id);
  }, [refresh, intervalMs]);

  return { status, refresh };
}
