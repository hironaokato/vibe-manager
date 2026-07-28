import { useEffect, useState, type FormEvent } from "react";
import { FolderOpen, Radar } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import type { Project, ProjectInput, StartupPolicy } from "../types";
import { Modal } from "./Modal";

interface ProjectFormProps {
  project?: Project;
  initialInput?: ProjectInput;
  defaultPolicy: StartupPolicy;
  busy: boolean;
  onClose: () => void;
  onSave: (input: ProjectInput) => Promise<void>;
}

export function ProjectForm({
  project,
  initialInput,
  defaultPolicy,
  busy,
  onClose,
  onSave,
}: ProjectFormProps) {
  const [name, setName] = useState(project?.name ?? initialInput?.name ?? "");
  const [directory, setDirectory] = useState(
    project?.directory ?? initialInput?.directory ?? "",
  );
  const [command, setCommand] = useState(
    project?.command ?? initialInput?.command ?? "",
  );
  const [url, setUrl] = useState(project?.url ?? initialInput?.url ?? "");
  const [startupPolicy, setStartupPolicy] = useState<StartupPolicy>(
    project?.startupPolicy ?? initialInput?.startupPolicy ?? defaultPolicy,
  );

  useEffect(() => {
    if (!project && directory && !name) {
      const pieces = directory.split(/[\\/]/).filter(Boolean);
      setName(pieces[pieces.length - 1] ?? "");
    }
  }, [directory, name, project]);

  async function chooseDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "プロジェクトの作業フォルダーを選択",
    });
    if (typeof selected === "string") setDirectory(selected);
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    void onSave({
      name,
      directory,
      command,
      url: url || undefined,
      startupPolicy,
      discoveryKey: initialInput?.discoveryKey ?? project?.discoveryKey,
      detectedPort: initialInput?.detectedPort ?? project?.detectedPort,
      externalPid: initialInput?.externalPid,
    });
  }

  return (
    <Modal
      title={project ? "プロジェクトを編集" : "プロジェクトを追加"}
      subtitle="実際にターミナルで使用している起動情報を登録します。"
      onClose={onClose}
    >
      <form className="project-form" onSubmit={submit}>
        {initialInput?.externalPid && (
          <div className="discovery-form-note">
            <Radar size={18} />
            <div>
              <strong>起動中のサーバーを取り込みます</strong>
              <p>
                PID {initialInput.externalPid} を監視対象に追加します。検出したコマンドは
                次回の起動にも使えるよう、必要に応じて編集してください。
              </p>
            </div>
          </div>
        )}

        <label className="field">
          <span>プロジェクト名</span>
          <input
            autoFocus
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Customer Portal"
            required
          />
        </label>

        <label className="field">
          <span>作業ディレクトリ</span>
          <div className="input-with-action">
            <input
              value={directory}
              onChange={(event) => setDirectory(event.target.value)}
              placeholder="C:\Users\name\GitHub\project"
              required
            />
            <button
              type="button"
              className="input-action"
              onClick={() => void chooseDirectory()}
              aria-label="フォルダーを選ぶ"
            >
              <FolderOpen size={18} />
            </button>
          </div>
        </label>

        <label className="field">
          <span>起動コマンド</span>
          <input
            className="mono"
            value={command}
            onChange={(event) => setCommand(event.target.value)}
            placeholder="npm run dev"
            required
          />
          <small>Windowsではcmd、macOSではzshを使って実行します。</small>
        </label>

        <label className="field">
          <span>
            ローカルURL <em>任意</em>
          </span>
          <input
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder="http://localhost:3000"
            type="url"
          />
        </label>

        <fieldset className="field policy-field">
          <legend>PC再起動後の扱い</legend>
          <div className="segmented-control">
            {(
              [
                ["auto", "自動復元"],
                ["ask", "確認する"],
                ["manual", "手動のみ"],
              ] as Array<[StartupPolicy, string]>
            ).map(([value, label]) => (
              <button
                type="button"
                key={value}
                className={startupPolicy === value ? "active" : ""}
                onClick={() => setStartupPolicy(value)}
              >
                {label}
              </button>
            ))}
          </div>
        </fieldset>

        <footer className="modal-actions">
          <button type="button" className="secondary-button" onClick={onClose}>
            キャンセル
          </button>
          <button type="submit" className="primary-button" disabled={busy}>
            {busy ? "保存しています…" : project ? "変更を保存" : "追加する"}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
