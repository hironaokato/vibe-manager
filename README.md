# Vibe Manager

Windowsの通知領域／macOSのメニューバーに常駐し、ローカル開発アプリをまとめて管理するデスクトップアプリです。

普段ターミナルで使っている作業ディレクトリとコマンドを登録すると、Vibe Managerから起動・停止・再起動でき、PC再起動後も「前回動いていたもの」を確認できます。

## 現在実装されている機能

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

## 状態の考え方

Vibe Managerは「実際のプロセス状態」と「ユーザーが動かしておきたい状態」を分けて保存します。

| 状態 | 意味 |
| --- | --- |
| 起動中 | Vibe Managerが起動した、または取り込んだプロセスが動作中 |
| 停止中 | ユーザーが明示的に停止 |
| 異常終了 | 動作中のプロセスが予期せず終了 |
| 復元待ち | 前回動いていたが、OS再起動などにより現在は停止 |

ウィンドウの閉じるボタンはアプリを終了せず、トレイへ隠します。トレイメニューからVibe Managerを完全終了する場合は、管理中のプロセスを安全に止め、次回の復元候補として状態を残します。

## localhostの自動検出

自動検出は既定で有効ですが、見つけただけのプロセスを勝手に停止・再起動することはありません。「自動検出」に候補として表示され、内容を確認して「取り込む」と監視対象になります。取り込んだプロセスが終了した場合は異常終了として残るため、「以前動いていたが今は落ちている」ことを確認できます。

確認を省きたい場合は、設定の「高確度の候補を自動登録」を有効にしてください。誤検出を減らすには「監視する作業フォルダー」へGitHubやProjectsフォルダーを1行ずつ登録するのがおすすめです。

WindowsではOSのTCP接続情報、macOSでは`lsof`から待受ポートとPIDを取得し、実行プロセスと作業ディレクトリを照合します。Docker、データベース、主要なWindowsシステムプロセスは候補から除外します。

## 技術構成

- UI: TypeScript、React、Vite
- デスクトップ基盤: Tauri 2
- 常駐・プロセス管理: Rust
- 状態保存: OSのアプリデータフォルダー内のJSON
- ログ: OSのアプリデータフォルダー内のプロジェクト別ファイル

保存先はTauriの標準アプリデータディレクトリです。

- Windows: `%APPDATA%\app.vibemanager.desktop`
- macOS: `~/Library/Application Support/app.vibemanager.desktop`

## 開発環境

### 共通

- Node.js 20以降
- Rust stable

### Windows

- Microsoft C++ Build Tools（Desktop development with C++）
- Microsoft Edge WebView2

### macOS

- Xcode Command Line Tools

```bash
xcode-select --install
```

## 開発コマンド

```bash
npm install
npm run tauri dev
```

フロントエンドのみの検査:

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

## 配布用ビルド

現在のOS向けインストーラーを生成します。

```bash
npm run tauri build
```

Windowsの配布用ビルドでは、必ず旧版削除を強制する専用コマンドを使います。

```powershell
npm run build:windows
```

Windows配布形式はNSISセットアップ（`-setup.exe`）だけに統一しています。異なるバージョンが検出された場合は旧版のアンインストーラーを実行し、完了を確認してから新しいファイルをコピーします。更新アンインストールでは、登録プロジェクト、設定、ログが入ったアプリデータを保持します。旧版が残っている場合はインストールを中止する二重チェックも入っています。

`.msi`は配布しません。インストール方式を混在させないため、過去に`.msi`で入れた環境もNSISセットアップが検出して先に削除します。

macOSでは、旧アプリバンドルを先に削除できるApple Silicon／Intel両対応のUniversal PKGを生成します。

```bash
npm run build:macos
```

PKGは実行中の旧版へ終了を要求し、終了できなければ更新を中止します。終了後に`/Applications/Vibe Manager.app`だけを削除して新しいバンドルを配置します。`~/Library/Application Support/app.vibemanager.desktop`は削除しないため、登録内容やログは引き継がれます。クリーン更新を保証できないDMGは配布しません。

Windows版はWindows上、macOS版はmacOS上でビルドしてください。署名なしのローカルビルドでは、OSのセキュリティ警告が表示される場合があります。

## プロセス管理上の注意

- 登録した起動コマンドはユーザー権限で実行されます。
- コマンドには秘密情報を直接書かず、プロジェクト側の環境変数や秘密管理を利用してください。
- Windowsでは`cmd.exe`、macOSでは`zsh`を使用します。
- Windowsではプロセスツリー、macOSではプロセスグループを単位として停止します。
- 取り込んだ外部起動プロセスを停止すると、そのプロセスも終了します。
- 検出した起動コマンドはOSのプロセス情報から復元するため、初回取り込み時に内容を確認してください。
- 管理対象プロジェクトのファイル自体をVibe Managerが削除することはありません。
