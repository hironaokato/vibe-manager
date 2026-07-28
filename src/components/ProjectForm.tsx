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
      title:
        "Select the project workspace folder / プロジェクトの作業フォルダーを選択",
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
      title={
        project
          ? "Edit project / プロジェクトを編集"
          : "Add project / プロジェクトを追加"
      }
      subtitle="Register the launch details you use in the terminal. / 実際にターミナルで使用している起動情報を登録します。"
      onClose={onClose}
    >
      <form className="project-form" onSubmit={submit}>
        {initialInput?.externalPid && (
          <div className="discovery-form-note">
            <Radar size={18} />
            <div>
              <strong>
                Importing a running server /
                起動中のサーバーを取り込みます
              </strong>
              <p>
                PID {initialInput.externalPid} will be added to monitoring.
                Review the detected command so it can be reused for the next
                launch. / PID {initialInput.externalPid} を監視対象に追加します。
                検出したコマンドは次回の起動にも使えるよう、必要に応じて編集してください。
              </p>
            </div>
          </div>
        )}

        <label className="field">
          <span>Project name / プロジェクト名</span>
          <input
            autoFocus
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Customer Portal"
            required
          />
        </label>

        <label className="field">
          <span>Working directory / 作業ディレクトリ</span>
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
              aria-label="Choose folder / フォルダーを選ぶ"
            >
              <FolderOpen size={18} />
            </button>
          </div>
        </label>

        <label className="field">
          <span>Launch command / 起動コマンド</span>
          <input
            className="mono"
            value={command}
            onChange={(event) => setCommand(event.target.value)}
            placeholder="npm run dev"
            required
          />
          <small>
            Runs with cmd on Windows and zsh on macOS. /
            Windowsではcmd、macOSではzshを使って実行します。
          </small>
        </label>

        <label className="field">
          <span>
            Local URL / ローカルURL <em>Optional / 任意</em>
          </span>
          <input
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder="http://localhost:3000"
            type="url"
          />
        </label>

        <fieldset className="field policy-field">
          <legend>After a PC restart / PC再起動後の扱い</legend>
          <div className="segmented-control">
            {(
              [
                ["auto", "Auto / 自動復元"],
                ["ask", "Ask / 確認する"],
                ["manual", "Manual / 手動のみ"],
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
            Cancel / キャンセル
          </button>
          <button type="submit" className="primary-button" disabled={busy}>
            {busy
              ? "Saving… / 保存しています…"
              : project
                ? "Save changes / 変更を保存"
                : "Add / 追加する"}
          </button>
        </footer>
      </form>
    </Modal>
  );
}
