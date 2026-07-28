import type { ProjectStatus, StartupPolicy } from "../types";

export const statusLabels: Record<ProjectStatus, string> = {
  running: "起動中",
  stopped: "停止中",
  starting: "起動しています",
  stopping: "停止しています",
  crashed: "異常終了",
  restorePending: "復元待ち",
};

export const policyLabels: Record<StartupPolicy, string> = {
  auto: "自動復元",
  ask: "確認して復元",
  manual: "手動のみ",
};

export function formatRelative(timestamp?: number, now = Date.now()): string {
  if (!timestamp) return "記録なし";
  const seconds = Math.max(0, Math.floor((now - timestamp) / 1000));
  if (seconds < 10) return "たった今";
  if (seconds < 60) return `${seconds}秒前`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}分前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}時間前`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}日前`;
  return new Intl.DateTimeFormat("ja-JP", {
    month: "short",
    day: "numeric",
  }).format(timestamp);
}

export function shortPath(path: string, max = 54): string {
  if (path.length <= max) return path;
  const separator = path.includes("\\") ? "\\" : "/";
  const pieces = path.split(separator);
  if (pieces.length < 3) return `…${path.slice(-(max - 1))}`;
  const result = `${pieces[0]}${separator}…${separator}${pieces
    .slice(-2)
    .join(separator)}`;
  return result.length <= max ? result : `…${path.slice(-(max - 1))}`;
}

export function canStart(status: ProjectStatus): boolean {
  return ["stopped", "crashed", "restorePending"].includes(status);
}

export function canStop(status: ProjectStatus): boolean {
  return status === "running";
}
