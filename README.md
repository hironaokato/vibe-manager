# Vibe Manager

[English](#english) | [日本語](#日本語)

## English

Vibe Manager is a desktop application that stays in the Windows system tray or macOS menu bar and manages local development applications in one place.

Register the working directories and commands you normally use in a terminal. Vibe Manager can then start, stop, and restart them while remembering what had been running before a PC restart.

The application displays English first, followed by Japanese.

### Features

- Windows system tray and macOS menu bar integration
- First-run setup and optional launch at login
- Add, edit, and remove projects
- Start, stop, and restart processes
- Distinct running, stopped, crashed, and restore-pending states
- PID, last start time, and exit code display
- Standard output and standard error capture
- Restore projects that were running in the previous session
- Per-project automatic, confirmation-based, or manual restore behavior
- Open a browser, folder, terminal, or VS Code
- Single-instance protection
- Project search and status filters
- Automatic discovery of unregistered development servers listening on localhost, about every 10 seconds
- Process detection for Node.js, Bun, Deno, Python, Rust, .NET, Java, PHP, and Ruby
- Review, open, import, or hide discovered candidates
- Optional automatic registration of high-confidence candidates
- Limit discovery to configured workspace folders
- Warning for servers exposed to the LAN through `0.0.0.0` or `::`

### Status model

Vibe Manager stores the actual process state separately from the state the user wants to maintain.

| Status | Meaning |
| --- | --- |
| Running | A process started or imported by Vibe Manager is running |
| Stopped | The user explicitly stopped the project |
| Crashed | A running process exited unexpectedly |
| Restore pending | The project was running previously but is currently stopped, for example after an OS restart |

Closing the window hides Vibe Manager in the tray instead of quitting it. When you quit from the tray or menu bar, managed processes are stopped safely and retained as candidates for the next restore.

### Automatic localhost discovery

Discovery is enabled by default, but Vibe Manager never stops or restarts a process merely because it was discovered. Candidates appear under **Discovery / 自動検出**. Review a candidate and choose **Import / 取り込む** to begin monitoring it.

If an imported process exits, it remains listed as crashed. This makes it possible to see that a server used to be running but is now down.

To skip confirmation, enable automatic registration of high-confidence candidates in Settings. To reduce false positives, add GitHub or Projects folders under the watched workspace folders, one folder per line.

On Windows, Vibe Manager reads the OS TCP connection table. On macOS, it uses `lsof`. It matches listening ports and PIDs with processes and working directories while excluding Docker, databases, and major Windows system processes.

### Technology

- UI: TypeScript, React, and Vite
- Desktop foundation: Tauri 2
- Background and process management: Rust
- State: JSON in the OS application data directory
- Logs: Per-project files in the OS application data directory

Data is stored in Tauri's standard application data directory:

- Windows: `%APPDATA%\app.vibemanager.desktop`
- macOS: `~/Library/Application Support/app.vibemanager.desktop`

### Development requirements

Common requirements:

- Node.js 20 or later
- Rust stable

Windows:

- Microsoft C++ Build Tools with **Desktop development with C++**
- Microsoft Edge WebView2

macOS:

- Xcode Command Line Tools

```bash
xcode-select --install
```

### Development commands

```bash
npm install
npm run tauri dev
```

Frontend checks:

```bash
npm run check
npm test
npm run build
```

Rust checks:

```bash
cd src-tauri
cargo fmt --check
cargo check
cargo test
```

### Distribution builds

For Windows, use the dedicated clean-upgrade command:

```powershell
npm run build:windows
```

Windows distribution uses only an NSIS setup executable (`-setup.exe`). The installer offers English first and Japanese second. When it detects a different installed version, it runs the previous uninstaller and verifies removal before copying new files. Project registrations, settings, and logs remain in the application data directory.

MSI packages are not distributed. The NSIS setup also detects and removes older MSI installations so installer technologies and versions are not mixed.

For macOS, build a Universal PKG supporting both Apple Silicon and Intel:

```bash
npm run build:macos
```

The PKG asks a running previous version to quit and cancels the update if it cannot exit. It then replaces only `/Applications/Vibe Manager.app`. It preserves `~/Library/Application Support/app.vibemanager.desktop`, including registrations and logs. A DMG is not distributed because it cannot guarantee the same clean-upgrade behavior.

Build the Windows package on Windows and the macOS package on macOS. Unsigned local builds may trigger operating-system security warnings.

### Process-management notes

- Registered commands run with the current user's permissions.
- Do not put secrets directly in commands. Use project environment variables or a secrets manager.
- Vibe Manager uses `cmd.exe` on Windows and `zsh` on macOS.
- It stops a process tree on Windows and a process group on macOS.
- Stopping an imported external process also terminates that process.
- A discovered launch command is reconstructed from OS process information, so review it during the first import.
- Vibe Manager never deletes files from managed projects.

---

## 日本語

Vibe Managerは、Windowsの通知領域／macOSのメニューバーに常駐し、ローカル開発アプリをまとめて管理するデスクトップアプリです。

普段ターミナルで使っている作業ディレクトリとコマンドを登録すると、Vibe Managerから起動・停止・再起動でき、PC再起動後も「前回動いていたもの」を確認できます。

アプリ内の文言は英語を先、日本語を後に表示します。

### 現在実装されている機能

- Windows通知領域／macOSメニューバーへの常駐
- 初回セットアップとログイン時の自動起動
- プロジェクトの登録・編集・削除
- 起動、停止、再起動
- 起動中、停止中、異常終了、復元待ちの区別
- PID、最終起動時刻、終了コードの表示
- 標準出力と標準エラーの保存・表示
- 前回起動していたプロジェクトの復元
- プロジェクトごとの自動復元／確認／手動設定
- ブラウザ、フォルダー、ターミナル、VS Codeを開く操作
- 二重起動の防止
- プロジェクト検索と状態フィルター
- localhostで待ち受ける未登録の開発サーバーを約10秒ごとに自動検出
- Node.js、Bun、Deno、Python、Rust、.NET、Java、PHP、Rubyの実行プロセス判定
- 検出候補の確認・ブラウザ表示・取り込み・非表示
- 高確度候補の自動登録（設定で明示的に有効化）
- 指定した作業フォルダー配下だけに自動検出を制限
- `0.0.0.0`や`::`で待ち受けているサーバーのLAN公開警告

### 状態の考え方

Vibe Managerは「実際のプロセス状態」と「ユーザーが動かしておきたい状態」を分けて保存します。

| 状態 | 意味 |
| --- | --- |
| 起動中 | Vibe Managerが起動した、または取り込んだプロセスが動作中 |
| 停止中 | ユーザーが明示的に停止 |
| 異常終了 | 動作中のプロセスが予期せず終了 |
| 復元待ち | 前回動いていたが、OS再起動などにより現在は停止 |

ウィンドウの閉じるボタンはアプリを終了せず、トレイへ隠します。トレイ／メニューバーから完全終了する場合は、管理中のプロセスを安全に止め、次回の復元候補として状態を残します。

### localhostの自動検出

自動検出は既定で有効ですが、見つけただけのプロセスを勝手に停止・再起動することはありません。**Discovery / 自動検出**に候補として表示され、内容を確認して**Import / 取り込む**を選ぶと監視対象になります。

取り込んだプロセスが終了した場合は異常終了として残るため、「以前動いていたが今は落ちている」ことを確認できます。

確認を省きたい場合は、設定の「高確度の候補を自動登録」を有効にしてください。誤検出を減らすには「監視する作業フォルダー」へGitHubやProjectsフォルダーを1行ずつ登録するのがおすすめです。

WindowsではOSのTCP接続情報、macOSでは`lsof`から待受ポートとPIDを取得し、実行プロセスと作業ディレクトリを照合します。Docker、データベース、主要なWindowsシステムプロセスは候補から除外します。

### 技術構成

- UI: TypeScript、React、Vite
- デスクトップ基盤: Tauri 2
- 常駐・プロセス管理: Rust
- 状態保存: OSのアプリデータフォルダー内のJSON
- ログ: OSのアプリデータフォルダー内のプロジェクト別ファイル

保存先はTauriの標準アプリデータディレクトリです。

- Windows: `%APPDATA%\app.vibemanager.desktop`
- macOS: `~/Library/Application Support/app.vibemanager.desktop`

### 開発環境

共通:

- Node.js 20以降
- Rust stable

Windows:

- Microsoft C++ Build Tools（Desktop development with C++）
- Microsoft Edge WebView2

macOS:

- Xcode Command Line Tools

```bash
xcode-select --install
```

### 開発コマンド

```bash
npm install
npm run tauri dev
```

フロントエンドの検査:

```bash
npm run check
npm test
npm run build
```

Rustの検査:

```bash
cd src-tauri
cargo fmt --check
cargo check
cargo test
```

### 配布用ビルド

Windowsでは、旧版削除を強制する専用コマンドを使います。

```powershell
npm run build:windows
```

Windows配布形式はNSISセットアップ（`-setup.exe`）だけに統一しています。インストーラーでは英語を先、日本語を後に選択できます。異なるバージョンが検出された場合は旧版のアンインストーラーを実行し、完了を確認してから新しいファイルをコピーします。登録プロジェクト、設定、ログが入ったアプリデータは保持します。

`.msi`は配布しません。インストール方式を混在させないため、過去に`.msi`で入れた環境もNSISセットアップが検出して先に削除します。

macOSでは、Apple Silicon／Intel両対応のUniversal PKGを生成します。

```bash
npm run build:macos
```

PKGは実行中の旧版へ終了を要求し、終了できなければ更新を中止します。終了後に`/Applications/Vibe Manager.app`だけを置換します。`~/Library/Application Support/app.vibemanager.desktop`は削除しないため、登録内容やログは引き継がれます。クリーン更新を保証できないDMGは配布しません。

Windows版はWindows上、macOS版はmacOS上でビルドしてください。署名なしのローカルビルドでは、OSのセキュリティ警告が表示される場合があります。

### プロセス管理上の注意

- 登録した起動コマンドはユーザー権限で実行されます。
- コマンドには秘密情報を直接書かず、プロジェクト側の環境変数や秘密管理を利用してください。
- Windowsでは`cmd.exe`、macOSでは`zsh`を使用します。
- Windowsではプロセスツリー、macOSではプロセスグループを単位として停止します。
- 取り込んだ外部起動プロセスを停止すると、そのプロセスも終了します。
- 検出した起動コマンドはOSのプロセス情報から復元するため、初回取り込み時に内容を確認してください。
- 管理対象プロジェクトのファイル自体をVibe Managerが削除することはありません。
