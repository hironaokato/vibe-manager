import { useEffect, useState } from "react";
import { Copy, Eraser, RefreshCw, TerminalSquare, X } from "lucide-react";
import { errorMessage, managerApi } from "../lib/manager-api";
import type { Project } from "../types";

interface LogsPanelProps {
  project: Project;
  onClose: () => void;
  onError: (message: string) => void;
}

export function LogsPanel({ project, onClose, onError }: LogsPanelProps) {
  const [logs, setLogs] = useState(
    "Loading logs… / ログを読み込んでいます…",
  );
  const [refreshing, setRefreshing] = useState(false);
  const [copied, setCopied] = useState(false);

  async function refresh() {
    setRefreshing(true);
    try {
      setLogs(await managerApi.logs(project.id));
    } catch (error) {
      onError(errorMessage(error));
    } finally {
      setRefreshing(false);
    }
  }

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 1_500);
    return () => window.clearInterval(interval);
    // The project id is the intended lifecycle boundary.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project.id]);

  async function copyLogs() {
    try {
      await navigator.clipboard.writeText(logs);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1_500);
    } catch (error) {
      onError(errorMessage(error));
    }
  }

  async function clearLogs() {
    try {
      await managerApi.clearLogs(project.id);
      await refresh();
    } catch (error) {
      onError(errorMessage(error));
    }
  }

  return (
    <aside
      className="logs-panel"
      aria-label={`Logs for ${project.name} / ${project.name}のログ`}
    >
      <header>
        <div className="logs-title">
          <span className="terminal-icon">
            <TerminalSquare size={17} />
          </span>
          <div>
            <strong>{project.name}</strong>
            <small>LIVE LOG</small>
          </div>
        </div>
        <div className="logs-actions">
          <button
            type="button"
            className="icon-button"
            title="Refresh / 再読み込み"
            onClick={() => void refresh()}
          >
            <RefreshCw size={16} className={refreshing ? "spin" : ""} />
          </button>
          <button
            type="button"
            className="icon-button"
            title="Copy / コピー"
            onClick={() => void copyLogs()}
          >
            <Copy size={16} />
          </button>
          <button
            type="button"
            className="icon-button"
            title="Clear logs / ログを消去"
            onClick={() => void clearLogs()}
          >
            <Eraser size={16} />
          </button>
          <button
            type="button"
            className="icon-button"
            title="Close / 閉じる"
            onClick={onClose}
          >
            <X size={17} />
          </button>
        </div>
      </header>
      {copied && (
        <div className="copied-toast">Copied / コピーしました</div>
      )}
      <pre>{logs}</pre>
      <footer>
        <span
          className={`live-dot ${project.status === "running" ? "" : "muted"}`}
        />
        {project.status === "running"
          ? "Auto-refreshing / 自動更新中"
          : "Process is stopped / プロセスは停止しています"}
      </footer>
    </aside>
  );
}
