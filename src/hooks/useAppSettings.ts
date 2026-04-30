import { useCallback, useEffect, useState } from "react";
import {
  getAppSettings,
  getProxyStatus,
  isAutostartEnabled,
} from "@/lib/tauri";
import type { AppSettings, ProxyStatus } from "@/lib/types";

export function useAppSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [proxyStatus, setProxyStatus] = useState<ProxyStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setError(null);
      const [nextSettings, nextAutostart, nextProxyStatus] = await Promise.all([
        getAppSettings(),
        isAutostartEnabled().catch(() => false),
        getProxyStatus().catch(() => ({ running: false, port: null })),
      ]);
      setSettings(nextSettings);
      setAutostartEnabled(nextAutostart);
      setProxyStatus(nextProxyStatus);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch app settings");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return {
    settings,
    setSettings,
    autostartEnabled,
    setAutostartEnabled,
    proxyStatus,
    loading,
    error,
    refresh,
  };
}
