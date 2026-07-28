import { useState } from "react";
import type { AppSettings, SettingsInput, StartupPolicy } from "../types";
import { Modal } from "./Modal";

interface SettingsModalProps {
  settings: AppSettings;
  ignoredDiscoveryCount: number;
  busy: boolean;
  onClose: () => void;
  onClearIgnored: () => Promise<void>;
  onSave: (settings: SettingsInput) => Promise<void>;
}

export function SettingsModal({
  settings,
  ignoredDiscoveryCount,
  busy,
  onClose,
  onClearIgnored,
  onSave,
}: SettingsModalProps) {
  const [launchAtLogin, setLaunchAtLogin] = useState(settings.launchAtLogin);
  const [discoveryEnabled, setDiscoveryEnabled] = useState(
    settings.discoveryEnabled,
  );
  const [autoRegisterDiscovered, setAutoRegisterDiscovered] = useState(
    settings.autoRegisterDiscovered,
  );
  const [workspaceRoots, setWorkspaceRoots] = useState(
    settings.workspaceRoots.join("\n"),
  );
  const [defaultPolicy, setDefaultPolicy] = useState<StartupPolicy>(
    settings.defaultStartupPolicy,
  );

  return (
    <Modal
      title="Settings / 設定"
      subtitle="Configure background behavior and defaults for new projects. / 常駐方法と、新しいプロジェクトの標準動作を変更します。"
      onClose={onClose}
    >
      <div className="settings-body">
        <section className="settings-section">
          <h3>System / システム</h3>
          <label className="settings-row">
            <span>
              <strong>
                Launch Vibe Manager at login / ログイン時にVibe Managerを起動
              </strong>
              <small>
                Runs in the tray without opening the window. /
                ウィンドウは開かず、トレイに常駐します。
              </small>
            </span>
            <input
              type="checkbox"
              checked={launchAtLogin}
              onChange={(event) => setLaunchAtLogin(event.target.checked)}
            />
            <span className="switch" />
          </label>
        </section>

        <section className="settings-section">
          <h3>Automatic discovery / 自動検出</h3>
          <label className="settings-row">
            <span>
              <strong>
                Discover local development servers /
                ローカル開発サーバーを検出
              </strong>
              <small>
                Checks listening ports and processes about every 10 seconds. /
                待受ポートとプロセスを約10秒ごとに確認します。
              </small>
            </span>
            <input
              type="checkbox"
              checked={discoveryEnabled}
              onChange={(event) => setDiscoveryEnabled(event.target.checked)}
            />
            <span className="switch" />
          </label>
          <label className={`settings-row ${!discoveryEnabled ? "disabled" : ""}`}>
            <span>
              <strong>
                Auto-register high-confidence candidates /
                高確度の候補を自動登録
              </strong>
              <small>
                Skips confirmation. Keeping this off initially is recommended. /
                確認画面を省きます。最初はオフがおすすめです。
              </small>
            </span>
            <input
              type="checkbox"
              checked={autoRegisterDiscovered}
              disabled={!discoveryEnabled}
              onChange={(event) =>
                setAutoRegisterDiscovered(event.target.checked)
              }
            />
            <span className="switch" />
          </label>
          <label className="field workspace-roots-field">
            <span>
              Workspace folders to watch / 監視する作業フォルダー{" "}
              <em>Optional / 任意</em>
            </span>
            <textarea
              value={workspaceRoots}
              onChange={(event) => setWorkspaceRoots(event.target.value)}
              placeholder={
                "One folder per line / 1行に1フォルダー\nC:\\Users\\name\\Documents\\GitHub\n/Users/name/Projects"
              }
              rows={3}
            />
            <small>
              When specified, only servers running under these folders become
              candidates. If left blank, development processes such as Node.js
              are detected. /
              指定すると、その配下で動くサーバーだけを候補にします。空欄ではNode.jsなどの開発用プロセスを判定します。
            </small>
          </label>
          {ignoredDiscoveryCount > 0 && (
            <div className="ignored-discovery-row">
              <span>
                Hidden candidates: {ignoredDiscoveryCount} /
                非表示にした候補: {ignoredDiscoveryCount}件
              </span>
              <button
                type="button"
                className="text-button"
                disabled={busy}
                onClick={() => void onClearIgnored()}
              >
                Reset hidden / 非表示をリセット
              </button>
            </div>
          )}
        </section>

        <section className="settings-section">
          <h3>New projects / 新規プロジェクト</h3>
          <label className="field">
            <span>Default restore behavior / 標準の復元方法</span>
            <select
              value={defaultPolicy}
              onChange={(event) =>
                setDefaultPolicy(event.target.value as StartupPolicy)
              }
            >
              <option value="ask">
                Ask before restoring / 確認してから復元
              </option>
              <option value="auto">Restore automatically / 自動で復元</option>
              <option value="manual">Always manual / 常に手動</option>
            </select>
          </label>
        </section>

        <section className="settings-note">
          <strong>
            Monitoring continues after closing the window /
            ウィンドウを閉じても監視は続きます
          </strong>
          <p>
            To quit completely, choose “Quit Vibe Manager / Vibe
            Managerを終了” from the tray or menu bar. /
            完全に終了する場合は、トレイ／メニューバーのメニューから終了を選択してください。
          </p>
        </section>
      </div>
      <footer className="modal-actions">
        <button type="button" className="secondary-button" onClick={onClose}>
          Cancel / キャンセル
        </button>
        <button
          type="button"
          className="primary-button"
          disabled={busy}
          onClick={() =>
            void onSave({
              onboardingComplete: true,
              launchAtLogin,
              defaultStartupPolicy: defaultPolicy,
              discoveryEnabled,
              autoRegisterDiscovered,
              workspaceRoots: Array.from(
                new Set(
                  workspaceRoots
                    .split(/\r?\n/)
                    .map((root) => root.trim())
                    .filter(Boolean),
                ),
              ),
            })
          }
        >
          {busy ? "Saving… / 保存しています…" : "Save settings / 設定を保存"}
        </button>
      </footer>
    </Modal>
  );
}
