import { useCallback, useEffect, useState } from "react";
import { getLogs } from "@/lib/tauri";
import type { RequestLog } from "@/lib/types";

interface UseLogsOptions {
  pageSize?: number;
}

export function useLogs(options: UseLogsOptions = {}) {
  const pageSize = options.pageSize ?? 16;
  const [logs, setLogs] = useState<RequestLog[]>([]);
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [selectedProviderId, setSelectedProviderId] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setError(null);
      const result = await getLogs({
        provider_id: selectedProviderId || undefined,
        date_from: dateFrom || undefined,
        date_to: dateTo || undefined,
        page,
        page_size: pageSize,
      });
      setLogs(result.logs);
      setTotal(result.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch logs");
    } finally {
      setLoading(false);
    }
  }, [dateFrom, dateTo, page, pageSize, selectedProviderId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const resetFilters = useCallback(() => {
    setSelectedProviderId("");
    setDateFrom("");
    setDateTo("");
    setPage(1);
  }, []);

  return {
    logs,
    page,
    setPage,
    total,
    totalPages: Math.max(1, Math.ceil(total / pageSize)),
    selectedProviderId,
    setSelectedProviderId,
    dateFrom,
    setDateFrom,
    dateTo,
    setDateTo,
    pageSize,
    loading,
    error,
    refresh,
    resetFilters,
  };
}
