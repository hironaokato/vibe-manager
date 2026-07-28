import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DashboardSnapshot,
  Project,
  ProjectInput,
  SettingsInput,
} from "../types";

export const managerApi = {
  snapshot: () => invoke<DashboardSnapshot>("get_snapshot"),
  saveSettings: (settings: SettingsInput) =>
    invoke<DashboardSnapshot>("save_settings", { settings }),
  createProject: (input: ProjectInput) =>
    invoke<Project>("create_project", { input }),
  updateProject: (id: string, input: ProjectInput) =>
    invoke<Project>("update_project", { id, input }),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),
  startProject: (id: string) => invoke<void>("start_project", { id }),
  stopProject: (id: string) => invoke<void>("stop_project", { id }),
  restartProject: (id: string) => invoke<void>("restart_project", { id }),
  restoreAll: () => invoke<string[]>("restore_all"),
  logs: (id: string) => invoke<string>("get_project_logs", { id }),
  clearLogs: (id: string) => invoke<void>("clear_project_logs", { id }),
  refreshDiscovery: () =>
    invoke<DashboardSnapshot>("refresh_discovery"),
  ignoreDiscovery: (key: string) =>
    invoke<void>("ignore_discovery_candidate", { key }),
  clearIgnoredDiscovery: () =>
    invoke<void>("clear_ignored_discovery_candidates"),
  openDiscoveredUrl: (key: string) =>
    invoke<void>("open_discovered_url", { key }),
  openDirectory: (id: string) =>
    invoke<void>("open_project_directory", { id }),
  openUrl: (id: string) => invoke<void>("open_project_url", { id }),
  openEditor: (id: string) => invoke<void>("open_project_editor", { id }),
  openTerminal: (id: string) =>
    invoke<void>("open_project_terminal", { id }),
  onSnapshot: (
    callback: (snapshot: DashboardSnapshot) => void,
  ): Promise<UnlistenFn> =>
    listen<DashboardSnapshot>("manager-state-changed", (event) =>
      callback(event.payload),
    ),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "予期しないエラーが発生しました。";
}
