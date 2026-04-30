import { useCallback, useEffect, useState } from "react";
import {
  createProvider,
  deleteProvider,
  listProviders,
  updateProvider,
} from "@/lib/tauri";
import type { Provider } from "@/lib/types";

export function useProviders() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setError(null);
      const nextProviders = await listProviders();
      setProviders(nextProviders);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch providers");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const create = useCallback(
    async (params: {
      name: string;
      base_url: string;
      api_key: string;
      protocol: string;
      model_mapping?: string;
      auth_header: string;
      keyword: string;
      enabled: boolean;
    }) => {
      await createProvider(params);
      await refresh();
    },
    [refresh]
  );

  const update = useCallback(
    async (params: {
      id: string;
      name: string;
      base_url: string;
      api_key?: string;
      protocol: string;
      model_mapping?: string;
      auth_header: string;
      keyword: string;
      enabled: boolean;
    }) => {
      await updateProvider(params);
      await refresh();
    },
    [refresh]
  );

  const remove = useCallback(
    async (id: string) => {
      await deleteProvider(id);
      await refresh();
    },
    [refresh]
  );

  return { providers, loading, error, refresh, create, update, remove };
}
