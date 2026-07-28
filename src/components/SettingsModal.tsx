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
      title="設定"
      subtitle="常駐方法と、新しいプロジェクトの標準動作を変更します。"
      onClose={onClose}
    >
      <div className="settings-body">
        <section className="settings-section">
          <h3>システム</h3>
          <label className="settings-row">
            <span>
              <strong>ログイン時にVibe Managerを起動</strong>
              <small>ウィンドウは開かず、トレイに常駐します。</small>
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
          <h3>自動検出</h3>
          <label className="settings-row">
            <span>
              <strong>ローカル開発サーバーを検出</strong>
              <small>待受ポートとプロセスを約10秒ごとに確認します。</small>
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
              <strong>高確度の候補を自動登録</strong>
              <small>確認画面を省きます。最初はオフがおすすめです。</small>
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
              監視する作業フォルダー <em>任意</em>
            </span>
            <textarea
              value={workspaceRoots}
              onChange={(event) => setWorkspaceRoots(event.target.value)}
              placeholder={
                "1行に1フォルダー\nC:\\Users\\name\\Documents\\GitHub\n/Users/name/Projects"
              }
              rows={3}
            />
            <small>
              指定すると、その配下で動くサーバーだけを候補にします。空欄ではNode.jsなどの開発用プロセスを判定します。
            </small>
          </label>
          {ignoredDiscoveryCount > 0 && (
            <div className="ignored-discovery-row">
              <span>非表示にした候補: {ignoredDiscoveryCount}件</span>
              <button
                type="button"
                className="text-button"
                disabled={busy}
                onClick={() => void onClearIgnored()}
              >
                非表示をリセット
              </button>
            </div>
          )}
        </section>

        <section className="settings-section">
          <h3>新規プロジェクト</h3>
          <label className="field">
            <span>標準の復元方法</span>
            <select
              value={defaultPolicy}
              onChange={(event) =>
                setDefaultPolicy(event.target.value as StartupPolicy)
              }
            >
              <option value="ask">確認してから復元</option>
              <option value="auto">自動で復元</option>
              <option value="manual">常に手動</option>
            </select>
          </label>
        </section>

        <section className="settings-note">
          <strong>ウィンドウを閉じても監視は続きます</strong>
          <p>
            完全に終了する場合は、トレイ／メニューバーのメニューから
            「Vibe Managerを終了」を選択してください。
          </p>
        </section>
      </div>
      <footer className="modal-actions">
        <button type="button" className="secondary-button" onClick={onClose}>
          キャンセル
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
          {busy ? "保存しています…" : "設定を保存"}
        </button>
      </footer>
    </Modal>
  );
}
