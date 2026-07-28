import { useCallback, useEffect, useState } from "react";
import { errorMessage, managerApi } from "../lib/manager-api";
import type { DashboardSnapshot } from "../types";

export function useManager() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await managerApi.snapshot();
      setSnapshot(next);
      setError(null);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const unlisten = managerApi.onSnapshot(setSnapshot);
    const interval = window.setInterval(() => void refresh(), 5_000);
    return () => {
      window.clearInterval(interval);
      void unlisten.then((stop) => stop());
    };
  }, [refresh]);

  return { snapshot, setSnapshot, loading, error, setError, refresh };
}
