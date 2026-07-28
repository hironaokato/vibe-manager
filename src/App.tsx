import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  AlertTriangle,
  AppWindow,
  Braces,
  Check,
  ChevronRight,
  CircleStop,
  Clock3,
  Code2,
  Command,
  ExternalLink,
  FileTerminal,
  FolderOpen,
  Github,
  LayoutDashboard,
  MoreHorizontal,
  Play,
  Plus,
  Radar,
  RefreshCw,
  RotateCcw,
  Search,
  Settings,
  SlidersHorizontal,
  SquarePen,
  TerminalSquare,
  Trash2,
} from "lucide-react";
import "./App.css";
import { LogsPanel } from "./components/LogsPanel";
import { Onboarding } from "./components/Onboarding";
import { ProjectForm } from "./components/ProjectForm";
import { SettingsModal } from "./components/SettingsModal";
import { useManager } from "./hooks/use-manager";
import {
  canStart,
  canStop,
  formatRelative,
  policyLabels,
  shortPath,
  statusLabels,
} from "./lib/format";
import { errorMessage, managerApi } from "./lib/manager-api";
import type {
  DiscoveryCandidate,
  Project,
  ProjectInput,
  SettingsInput,
} from "./types";

type Filter = "all" | "running" | "attention" | "stopped" | "discovered";

function App() {
  const { snapshot, setSnapshot, loading, error, setError, refresh } =
    useManager();
  const [filter, setFilter] = useState<Filter>("all");
  const [search, setSearch] = useState("");
  const [editingProject, setEditingProject] = useState<Project | null>();
  const [discoveryDraft, setDiscoveryDraft] =
    useState<DiscoveryCandidate | null>(null);
  const [logsProjectId, setLogsProjectId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const isMac = navigator.userAgent.toLocaleLowerCase().includes("mac");

  useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchInput.current?.focus();
      }
    };
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, []);

  const projects = useMemo(() => {
    if (!snapshot) return [];
    const term = search.trim().toLocaleLowerCase();
    return snapshot.projects
      .filter((project) => {
        if (filter === "running") return project.status === "running";
        if (filter === "attention")
          return ["crashed", "restorePending"].includes(project.status);
        if (filter === "stopped") return project.status === "stopped";
        return true;
      })
      .filter(
        (project) =>
          !term ||
          project.name.toLocaleLowerCase().includes(term) ||
          project.directory.toLocaleLowerCase().includes(term) ||
          project.command.toLocaleLowerCase().includes(term),
      )
      .sort((a, b) => {
        const priority = {
          crashed: 0,
          restorePending: 1,
          running: 2,
          starting: 3,
          stopping: 4,
          stopped: 5,
        };
        return priority[a.status] - priority[b.status];
      });
  }, [filter, search, snapshot]);

  const logsProject = snapshot?.projects.find(
    (project) => project.id === logsProjectId,
  );

  const discoveryCandidates = useMemo(() => {
    if (!snapshot) return [];
    const term = search.trim().toLocaleLowerCase();
    return snapshot.discoveryCandidates.filter(
      (candidate) =>
        !term ||
        candidate.name.toLocaleLowerCase().includes(term) ||
        candidate.directory.toLocaleLowerCase().includes(term) ||
        candidate.command.toLocaleLowerCase().includes(term) ||
        String(candidate.port).includes(term),
    );
  }, [search, snapshot]);

  function showToast(message: string) {
    setToast(message);
    window.setTimeout(() => setToast(null), 2_400);
  }

  async function perform(
    key: string,
    action: () => Promise<unknown>,
    success?: string,
  ) {
    setBusyKey(key);
    setError(null);
    try {
      await action();
      await refresh();
      if (success) showToast(success);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusyKey(null);
    }
  }

  async function saveSettings(settings: SettingsInput) {
    await perform(
      "settings",
      async () => {
        const next = await managerApi.saveSettings(settings);
        setSnapshot(next);
      },
      "Settings saved / 設定を保存しました",
    );
  }

  async function saveProject(input: ProjectInput) {
    const editing = editingProject ?? undefined;
    const adopting = discoveryDraft !== null;
    await perform(
      "project-form",
      async () => {
        if (editing) {
          await managerApi.updateProject(editing.id, input);
        } else {
          await managerApi.createProject(input);
        }
        setEditingProject(undefined);
        setDiscoveryDraft(null);
      },
      editing
        ? "Project updated / プロジェクトを更新しました"
        : adopting
          ? "Running server imported / 起動中のサーバーを取り込みました"
          : "Project added / プロジェクトを追加しました",
    );
  }

  async function refreshDiscovery() {
    await perform(
      "discovery-scan",
      async () => {
        const next = await managerApi.refreshDiscovery();
        setSnapshot(next);
      },
      "Local servers rescanned / ローカルサーバーを再スキャンしました",
    );
  }

  async function ignoreDiscovery(candidate: DiscoveryCandidate) {
    await perform(
      `ignore-${candidate.key}`,
      () => managerApi.ignoreDiscovery(candidate.key),
      `${candidate.name} hidden from candidates / ${candidate.name} を候補から非表示にしました`,
    );
  }

  if (loading || !snapshot) {
    return (
      <main className="loading-screen">
        <div className="brand-mark large">
          <Command size={26} />
          <span />
        </div>
        <p>Preparing Vibe Manager… / Vibe Managerを準備しています…</p>
      </main>
    );
  }

  if (!snapshot.settings.onboardingComplete) {
    return (
      <>
        <Onboarding
          busy={busyKey === "settings"}
          onComplete={saveSettings}
        />
        {error && <ErrorToast message={error} onClose={() => setError(null)} />}
      </>
    );
  }

  return (
    <div className={`app-shell ${logsProject ? "with-logs" : ""}`}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <Command size={20} strokeWidth={2.5} />
            <span />
          </div>
          <div>
            <strong>Vibe Manager</strong>
            <small>LOCAL RUNTIME</small>
          </div>
        </div>

        <nav>
          <p>WORKSPACE</p>
          <button
            type="button"
            className={filter === "all" ? "active" : ""}
            onClick={() => setFilter("all")}
          >
            <LayoutDashboard size={17} />
            All apps / すべてのアプリ
            <span>{snapshot.projects.length}</span>
          </button>
          <button
            type="button"
            className={filter === "running" ? "active" : ""}
            onClick={() => setFilter("running")}
          >
            <Play size={16} />
            Running / 起動中
            <span>{snapshot.runningCount}</span>
          </button>
          <button
            type="button"
            className={filter === "attention" ? "active" : ""}
            onClick={() => setFilter("attention")}
          >
            <AlertTriangle size={16} />
            Attention / 確認が必要
            <span>{snapshot.crashedCount + snapshot.restoreCount}</span>
          </button>
          <button
            type="button"
            className={filter === "stopped" ? "active" : ""}
            onClick={() => setFilter("stopped")}
          >
            <CircleStop size={16} />
            Stopped / 停止中
          </button>
          <button
            type="button"
            aria-label="Automatic discovery / 自動検出"
            className={filter === "discovered" ? "active" : ""}
            onClick={() => setFilter("discovered")}
          >
            <Radar size={16} />
            Discovery / 自動検出
            <span>{snapshot.discoveryCount}</span>
          </button>
        </nav>

        <div className="sidebar-status">
          <div className="pulse-indicator">
            <span />
          </div>
          <div>
            <strong>Manager is active</strong>
            <small>
              Monitoring continues when closed /
              ウィンドウを閉じても監視します
            </small>
          </div>
        </div>
        <button
          type="button"
          className="sidebar-settings"
          onClick={() => setSettingsOpen(true)}
        >
          <Settings size={17} />
          Settings / 設定
          <ChevronRight size={15} />
        </button>
      </aside>

      <main className="dashboard">
        <header className="topbar">
          <div>
            <p className="eyebrow">LOCAL WORKSPACE</p>
            <h1>Applications / アプリケーション</h1>
          </div>
          <div className="topbar-actions">
            <div className="search-box">
              <Search size={16} />
              <input
                ref={searchInput}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Search by name, command, or path / 名前、コマンド、パスで検索"
              />
              <kbd>{isMac ? "⌘ K" : "Ctrl K"}</kbd>
            </div>
            <button
              type="button"
              className="primary-button add-button"
              onClick={() => setEditingProject(null)}
            >
              <Plus size={17} />
              Add / 追加
            </button>
          </div>
        </header>

        {snapshot.restoreCount > 0 && (
          <section className="restore-banner">
            <div className="restore-icon">
              <RotateCcw size={20} />
            </div>
            <div>
              <strong>
                {snapshot.restoreCount} previously running app(s) can be
                restored / 前回動いていたアプリが{snapshot.restoreCount}件あります
              </strong>
              <p>
                Review the list and restore what you need. /
                内容を確認して、必要なものを復元できます。
              </p>
            </div>
            <button
              type="button"
              className="banner-button"
              disabled={busyKey === "restore-all"}
              onClick={() =>
                void perform(
                  "restore-all",
                  managerApi.restoreAll,
                  "Restore started / 復元処理を開始しました",
                )
              }
            >
              {busyKey === "restore-all"
                ? "Restoring… / 復元中…"
                : "Restore all / すべて復元"}
            </button>
          </section>
        )}

        <section className="metric-grid">
          <MetricCard
            icon={<Play size={17} />}
            label="RUNNING"
            value={snapshot.runningCount}
            detail="Currently running / 現在起動中"
            tone="green"
          />
          <MetricCard
            icon={<RotateCcw size={17} />}
            label="RESTORE"
            value={snapshot.restoreCount}
            detail="Pending from last session / 前回から復元待ち"
            tone="violet"
          />
          <MetricCard
            icon={<AlertTriangle size={17} />}
            label="ATTENTION"
            value={snapshot.crashedCount}
            detail="Review crashed apps / 異常終了を確認"
            tone="red"
          />
          <MetricCard
            icon={<Radar size={17} />}
            label="DISCOVERED"
            value={snapshot.discoveryCount}
            detail="Unregistered candidates / 未登録の候補"
            tone="blue"
          />
        </section>

        {(filter === "discovered" || snapshot.discoveryCount > 0) && (
          <DiscoverySection
            candidates={discoveryCandidates}
            expanded={filter === "discovered"}
            scanning={busyKey === "discovery-scan"}
            busyKey={busyKey}
            onScan={() => void refreshDiscovery()}
            onOpen={(candidate) =>
              void perform(
                `open-${candidate.key}`,
                () => managerApi.openDiscoveredUrl(candidate.key),
              )
            }
            onIgnore={(candidate) => void ignoreDiscovery(candidate)}
            onShowAll={() => setFilter("discovered")}
            onImport={(candidate) => {
              setEditingProject(undefined);
              setDiscoveryDraft(candidate);
            }}
          />
        )}

        {filter !== "discovered" && (
        <section className="project-section">
          <header className="section-heading">
            <div>
              <h2>
                {filter === "all"
                  ? "All apps / すべてのアプリ"
                  : filter === "running"
                    ? "Running apps / 起動中のアプリ"
                  : filter === "attention"
                      ? "Apps needing attention / 確認が必要なアプリ"
                      : filter === "stopped"
                        ? "Stopped apps / 停止中のアプリ"
                        : "All apps / すべてのアプリ"}
              </h2>
              <span>{projects.length} item(s) / {projects.length}件</span>
            </div>
            <button
              type="button"
              className="icon-button"
              title="Refresh / 再読み込み"
              onClick={() => void refresh()}
            >
              <RefreshCw size={16} />
            </button>
          </header>

          {projects.length === 0 ? (
            <EmptyState
              hasProjects={snapshot.projects.length > 0}
              onAdd={() => setEditingProject(null)}
            />
          ) : (
            <div className="project-list">
              {projects.map((project) => (
                <ProjectRow
                  key={project.id}
                  project={project}
                  busy={busyKey === project.id}
                  onAction={(action) => {
                    const actions = {
                      start: () => managerApi.startProject(project.id),
                      stop: () => managerApi.stopProject(project.id),
                      restart: () => managerApi.restartProject(project.id),
                    };
                    const messages = {
                      start: `${project.name} started / ${project.name}を起動しました`,
                      stop: `${project.name} stopped / ${project.name}を停止しました`,
                      restart: `${project.name} restarted / ${project.name}を再起動しました`,
                    };
                    void perform(project.id, actions[action], messages[action]);
                  }}
                  onLogs={() => setLogsProjectId(project.id)}
                  onOpen={(target) => {
                    const actions = {
                      url: () => managerApi.openUrl(project.id),
                      directory: () => managerApi.openDirectory(project.id),
                      terminal: () => managerApi.openTerminal(project.id),
                      editor: () => managerApi.openEditor(project.id),
                    };
                    void perform(`${project.id}-${target}`, actions[target]);
                  }}
                  onEdit={() => setEditingProject(project)}
                  onDelete={() => {
                    if (
                      window.confirm(
                        `Remove “${project.name}” from Vibe Manager?\nThe project files will not be deleted.\n\n「${project.name}」をVibe Managerから削除しますか？\nプロジェクトのファイル自体は削除されません。`,
                      )
                    ) {
                      void perform(
                        project.id,
                        () => managerApi.deleteProject(project.id),
                        "Registration removed / 登録を削除しました",
                      );
                    }
                  }}
                />
              ))}
            </div>
          )}
        </section>
        )}
      </main>

      {logsProject && (
        <LogsPanel
          project={logsProject}
          onClose={() => setLogsProjectId(null)}
          onError={setError}
        />
      )}

      {(editingProject !== undefined || discoveryDraft) && (
        <ProjectForm
          project={editingProject ?? undefined}
          initialInput={
            discoveryDraft
              ? {
                  name: discoveryDraft.name,
                  directory: discoveryDraft.directory,
                  command: discoveryDraft.command,
                  url: discoveryDraft.url,
                  startupPolicy: snapshot.settings.defaultStartupPolicy,
                  discoveryKey: discoveryDraft.key,
                  detectedPort: discoveryDraft.port,
                  externalPid: discoveryDraft.pid,
                }
              : undefined
          }
          defaultPolicy={snapshot.settings.defaultStartupPolicy}
          busy={busyKey === "project-form"}
          onClose={() => {
            setEditingProject(undefined);
            setDiscoveryDraft(null);
          }}
          onSave={saveProject}
        />
      )}

      {settingsOpen && (
        <SettingsModal
          settings={snapshot.settings}
          ignoredDiscoveryCount={snapshot.ignoredDiscoveryCount}
          busy={busyKey === "settings"}
          onClose={() => setSettingsOpen(false)}
          onClearIgnored={async () => {
            await perform(
              "settings",
              managerApi.clearIgnoredDiscovery,
              "Hidden candidates reset / 非表示にした候補をリセットしました",
            );
          }}
          onSave={async (settings) => {
            await saveSettings(settings);
            setSettingsOpen(false);
          }}
        />
      )}

      {error && <ErrorToast message={error} onClose={() => setError(null)} />}
      {toast && (
        <div className="success-toast">
          <Check size={16} /> {toast}
        </div>
      )}
    </div>
  );
}

interface DiscoverySectionProps {
  candidates: DiscoveryCandidate[];
  expanded: boolean;
  scanning: boolean;
  busyKey: string | null;
  onScan: () => void;
  onOpen: (candidate: DiscoveryCandidate) => void;
  onIgnore: (candidate: DiscoveryCandidate) => void;
  onShowAll: () => void;
  onImport: (candidate: DiscoveryCandidate) => void;
}

function DiscoverySection({
  candidates,
  expanded,
  scanning,
  busyKey,
  onScan,
  onOpen,
  onIgnore,
  onShowAll,
  onImport,
}: DiscoverySectionProps) {
  const visibleCandidates = expanded ? candidates : candidates.slice(0, 3);

  return (
    <section className={`discovery-section ${expanded ? "expanded" : ""}`}>
      <header className="section-heading discovery-heading">
        <div>
          <div className="discovery-title-icon">
            <Radar size={16} />
          </div>
          <div className="discovery-title">
            <h2>Running unregistered servers / 起動中の未登録サーバー</h2>
            <p>
              Detected from listening ports and running processes. Vibe Manager
              will not stop or restart them until imported. /
              待受ポートと実行プロセスから検出しました。取り込むまでは停止・再起動しません。
            </p>
          </div>
          <span>
            {candidates.length} item(s) / {candidates.length}件
          </span>
        </div>
        <button
          type="button"
          className="secondary-button discovery-scan-button"
          disabled={scanning}
          onClick={onScan}
        >
          <RefreshCw size={14} className={scanning ? "spin" : ""} />
          {scanning ? "Scanning… / スキャン中" : "Rescan / 再スキャン"}
        </button>
      </header>

      {visibleCandidates.length === 0 ? (
        <div className="discovery-empty">
          <Radar size={23} />
          <strong>
            No unregistered development servers found /
            未登録の開発サーバーは見つかりませんでした
          </strong>
          <span>
            Start a server and rescan, or check the watched folders in
            Settings. /
            サーバーを起動してから再スキャンするか、設定で監視フォルダーを確認してください。
          </span>
        </div>
      ) : (
        <div className="discovery-list">
          {visibleCandidates.map((candidate) => (
            <article className="discovery-card" key={candidate.key}>
              <div className="candidate-port">
                <Radar size={18} />
                <strong>:{candidate.port}</strong>
              </div>
              <div className="candidate-details">
                <div>
                  <h3>{candidate.name}</h3>
                  <span>{candidate.processType}</span>
                  <span>
                    Confidence {candidate.confidence}% / 確度{" "}
                    {candidate.confidence}%
                  </span>
                  {candidate.externalExposure && (
                    <span className="exposure-badge">
                      LAN exposed / LAN公開
                    </span>
                  )}
                </div>
                <p title={candidate.directory}>
                  <FolderOpen size={12} />
                  {candidate.directory ||
                    "Workspace unavailable / 作業フォルダーを取得できませんでした"}
                </p>
                <code title={candidate.command}>{candidate.command}</code>
              </div>
              <div className="candidate-process">
                <span>PID {candidate.pid}</span>
                <small>{candidate.processName}</small>
              </div>
              <div className="candidate-actions">
                <button
                  type="button"
                  className="icon-button"
                  title={`Open ${candidate.url} / ${candidate.url} を開く`}
                  onClick={() => onOpen(candidate)}
                >
                  <ExternalLink size={15} />
                </button>
                <button
                  type="button"
                  className="text-button"
                  disabled={busyKey === `ignore-${candidate.key}`}
                  onClick={() => onIgnore(candidate)}
                >
                  Hide / 非表示
                </button>
                <button
                  type="button"
                  className="primary-button candidate-import-button"
                  onClick={() => onImport(candidate)}
                >
                  Import / 取り込む
                </button>
              </div>
            </article>
          ))}
        </div>
      )}

      {!expanded && candidates.length > visibleCandidates.length && (
        <button
          type="button"
          className="discovery-more-button"
          onClick={onShowAll}
        >
          Show {candidates.length - visibleCandidates.length} more / ほか{" "}
          {candidates.length - visibleCandidates.length}件を表示
          <ChevronRight size={14} />
        </button>
      )}
    </section>
  );
}

interface MetricCardProps {
  icon: ReactNode;
  label: string;
  value: number;
  detail: string;
  tone: string;
}

function MetricCard({ icon, label, value, detail, tone }: MetricCardProps) {
  return (
    <article className={`metric-card metric-${tone}`}>
      <div className="metric-icon">{icon}</div>
      <div>
        <p>{label}</p>
        <strong>{value}</strong>
        <span>{detail}</span>
      </div>
    </article>
  );
}

interface ProjectRowProps {
  project: Project;
  busy: boolean;
  onAction: (action: "start" | "stop" | "restart") => void;
  onLogs: () => void;
  onOpen: (target: "url" | "directory" | "terminal" | "editor") => void;
  onEdit: () => void;
  onDelete: () => void;
}

function ProjectRow({
  project,
  busy,
  onAction,
  onLogs,
  onOpen,
  onEdit,
  onDelete,
}: ProjectRowProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const latestTime =
    project.status === "running"
      ? project.lastStartedAt
      : (project.lastStoppedAt ?? project.updatedAt);

  return (
    <article className={`project-row status-${project.status}`}>
      <div className="project-main">
        <div className="project-logo">
          {project.command.includes("npm") ||
          project.command.includes("pnpm") ||
          project.command.includes("yarn") ? (
            <Braces size={21} />
          ) : project.command.includes("python") ? (
            <Code2 size={21} />
          ) : (
            <AppWindow size={21} />
          )}
          <span className={`status-dot status-dot-${project.status}`} />
        </div>
        <div className="project-identity">
          <div>
            <h3>{project.name}</h3>
            <span className={`status-badge badge-${project.status}`}>
              {statusLabels[project.status]}
            </span>
            {project.status === "running" &&
              project.processOrigin === "external" && (
                <span className="status-badge badge-external">
                  External / 外部起動
                </span>
              )}
          </div>
          <p title={project.directory}>
            <FolderOpen size={13} /> {shortPath(project.directory)}
          </p>
        </div>
      </div>

      <div className="project-command">
        <code>{project.command}</code>
        <span>{policyLabels[project.startupPolicy]}</span>
      </div>

      <div className="project-time">
        <Clock3 size={14} />
        <span>
          {project.status === "running"
            ? "Started / 起動 "
            : "Updated / 更新 "}
          {formatRelative(latestTime)}
        </span>
        {project.pid && <small>PID {project.pid}</small>}
      </div>

      <div className="project-tools">
        {project.url && (
          <button
            type="button"
            className="icon-button"
            title="Open in browser / ブラウザで開く"
            onClick={() => onOpen("url")}
          >
            <ExternalLink size={16} />
          </button>
        )}
        <button
          type="button"
          className="icon-button"
          title="View logs / ログを見る"
          onClick={onLogs}
        >
          <FileTerminal size={16} />
        </button>
        <div className="more-menu-wrap">
          <button
            type="button"
            className="icon-button"
            title="More / その他"
            onClick={() => setMenuOpen((open) => !open)}
          >
            <MoreHorizontal size={18} />
          </button>
          {menuOpen && (
            <div className="more-menu" onMouseLeave={() => setMenuOpen(false)}>
              <button type="button" onClick={() => onOpen("directory")}>
                <FolderOpen size={15} /> Open folder / フォルダーを開く
              </button>
              <button type="button" onClick={() => onOpen("terminal")}>
                <TerminalSquare size={15} /> Open terminal / ターミナルを開く
              </button>
              <button type="button" onClick={() => onOpen("editor")}>
                <Github size={15} /> Open in VS Code / VS Codeで開く
              </button>
              <button type="button" onClick={onEdit}>
                <SquarePen size={15} /> Edit / 編集
              </button>
              <button
                type="button"
                className="danger"
                onClick={onDelete}
                disabled={project.status === "running"}
              >
                <Trash2 size={15} /> Remove / 登録を削除
              </button>
            </div>
          )}
        </div>
      </div>

      <div className="project-action">
        {canStop(project.status) ? (
          <>
            <button
              type="button"
              className="restart-button"
              title="Restart / 再起動"
              disabled={busy}
              onClick={() => onAction("restart")}
            >
              <RefreshCw size={15} />
            </button>
            <button
              type="button"
              className="stop-button"
              disabled={busy}
              onClick={() => onAction("stop")}
            >
              <CircleStop size={15} />
              {busy ? "Working… / 処理中" : "Stop / 停止"}
            </button>
          </>
        ) : canStart(project.status) ? (
          <button
            type="button"
            className={`start-button ${
              project.status === "crashed" ? "recover" : ""
            }`}
            disabled={busy}
            onClick={() => onAction("start")}
          >
            {project.status === "crashed" ? (
              <RefreshCw size={15} />
            ) : (
              <Play size={15} />
            )}
            {busy
              ? "Working… / 処理中"
              : project.status === "crashed"
                ? "Restart / 再起動"
                : "Start / 起動"}
          </button>
        ) : (
          <button type="button" className="pending-button" disabled>
            <RefreshCw size={15} className="spin" /> Working… / 処理中
          </button>
        )}
      </div>

      {project.lastError && project.status === "crashed" && (
        <div className="project-error">
          <AlertTriangle size={14} />
          {project.lastError}
          {project.lastExitCode !== undefined && (
            <span>
              Exit code {project.lastExitCode} / 終了コード{" "}
              {project.lastExitCode}
            </span>
          )}
        </div>
      )}
    </article>
  );
}

function EmptyState({
  hasProjects,
  onAdd,
}: {
  hasProjects: boolean;
  onAdd: () => void;
}) {
  return (
    <div className="empty-state">
      <div>
        <SlidersHorizontal size={25} />
      </div>
      <h3>
        {hasProjects
          ? "No apps match / 条件に一致するアプリがありません"
          : "Add your first app / 最初のアプリを登録しましょう"}
      </h3>
      <p>
        {hasProjects
          ? "Change the filter or search terms. / フィルターまたは検索条件を変更してください。"
          : "Register the folder and command you normally use in the terminal. / 普段ターミナルで実行しているフォルダーとコマンドを登録します。"}
      </p>
      {!hasProjects && (
        <button type="button" className="primary-button" onClick={onAdd}>
          <Plus size={16} /> Add project / プロジェクトを追加
        </button>
      )}
    </div>
  );
}

function ErrorToast({
  message,
  onClose,
}: {
  message: string;
  onClose: () => void;
}) {
  return (
    <button type="button" className="error-toast" onClick={onClose}>
      <AlertTriangle size={17} />
      <span>{message}</span>
      <small>Close / 閉じる</small>
    </button>
  );
}

export default App;
