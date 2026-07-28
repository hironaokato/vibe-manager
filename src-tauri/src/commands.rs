use crate::{
    manager::AppState,
    models::{AppSettings, DashboardSnapshot, ProjectInput, ProjectRecord, SettingsInput},
};
#[cfg(not(any(windows, target_os = "macos")))]
use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> DashboardSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: SettingsInput,
) -> Result<DashboardSnapshot, String> {
    let settings = AppSettings::from(settings);
    if settings.launch_at_login {
        app.autolaunch()
            .enable()
            .map_err(|error| {
                format!(
                    "Could not enable launch at login: {error} / ログイン時起動を有効にできません: {error}"
                )
            })?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|error| {
                format!(
                    "Could not disable launch at login: {error} / ログイン時起動を無効にできません: {error}"
                )
            })?;
    }
    state.save_settings(&app, settings)
}

#[tauri::command]
pub fn create_project(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ProjectInput,
) -> Result<ProjectRecord, String> {
    state.create_project(&app, input)
}

#[tauri::command]
pub fn update_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: ProjectInput,
) -> Result<ProjectRecord, String> {
    state.update_project(&app, &id, input)
}

#[tauri::command]
pub fn delete_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.delete_project(&app, &id)
}

#[tauri::command]
pub fn start_project(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.start_project(&app, &id)
}

#[tauri::command]
pub fn stop_project(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.stop_project(&app, &id)
}

#[tauri::command]
pub fn restart_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.restart_project(&app, &id)
}

#[tauri::command]
pub fn restore_all(app: AppHandle, state: State<'_, AppState>) -> Vec<String> {
    state.restore_all(&app)
}

#[tauri::command]
pub fn get_project_logs(state: State<'_, AppState>, id: String) -> Result<String, String> {
    state.get_logs(&id)
}

#[tauri::command]
pub fn clear_project_logs(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.clear_logs(&id)
}

#[tauri::command]
pub fn refresh_discovery(app: AppHandle, state: State<'_, AppState>) -> DashboardSnapshot {
    state.refresh_discovery(&app);
    state.snapshot()
}

#[tauri::command]
pub fn ignore_discovery_candidate(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
) -> Result<(), String> {
    state.ignore_discovery_candidate(&app, &key)
}

#[tauri::command]
pub fn clear_ignored_discovery_candidates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.clear_ignored_discovery_candidates(&app)
}

#[tauri::command]
pub fn open_discovered_url(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
) -> Result<(), String> {
    let candidate = state
        .discovery_candidate(&key)
        .ok_or_else(|| {
            "Discovery candidate not found. Please rescan. / 検出候補が見つかりません。再スキャンしてください。"
                .to_string()
        })?;
    app.opener()
        .open_url(candidate.url, None::<&str>)
        .map_err(|error| format!("Could not open URL: {error} / URLを開けません: {error}"))
}

#[tauri::command]
pub fn open_project_directory(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let project = project_by_id(&state.snapshot(), &id)?;
    app.opener()
        .open_path(project.directory, None::<&str>)
        .map_err(|error| {
            format!("Could not open folder: {error} / フォルダーを開けません: {error}")
        })
}

#[tauri::command]
pub fn open_project_url(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let project = project_by_id(&state.snapshot(), &id)?;
    let url = project.url.ok_or_else(|| {
        "No URL is registered for this project. / このプロジェクトにはURLが登録されていません。"
            .to_string()
    })?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| format!("Could not open URL: {error} / URLを開けません: {error}"))
}

#[tauri::command]
pub fn open_project_editor(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let project = project_by_id(&state.snapshot(), &id)?;
    spawn_editor(&project.directory)
}

#[tauri::command]
pub fn open_project_terminal(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let project = project_by_id(&state.snapshot(), &id)?;
    spawn_terminal(&project.directory)
}

fn project_by_id(snapshot: &DashboardSnapshot, id: &str) -> Result<ProjectRecord, String> {
    snapshot
        .projects
        .iter()
        .find(|project| project.id == id)
        .cloned()
        .ok_or_else(|| "Project not found. / プロジェクトが見つかりません。".to_string())
}

#[cfg(windows)]
fn spawn_terminal(directory: &str) -> Result<(), String> {
    if Command::new("wt.exe")
        .args(["-d", directory])
        .spawn()
        .is_ok()
    {
        return Ok(());
    }
    Command::new("powershell.exe")
        .current_dir(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            format!("Could not open terminal: {error} / ターミナルを開けません: {error}")
        })
}

#[cfg(target_os = "macos")]
fn spawn_terminal(directory: &str) -> Result<(), String> {
    Command::new("open")
        .args(["-a", "Terminal", directory])
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            format!("Could not open terminal: {error} / ターミナルを開けません: {error}")
        })
}

#[cfg(not(any(windows, target_os = "macos")))]
fn spawn_terminal(directory: &str) -> Result<(), String> {
    Command::new("x-terminal-emulator")
        .current_dir(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            format!("Could not open terminal: {error} / ターミナルを開けません: {error}")
        })
}

#[cfg(windows)]
fn spawn_editor(directory: &str) -> Result<(), String> {
    Command::new("code.cmd")
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            format!(
                "Could not open VS Code. Check the code command: {error} / VS Codeを開けません。codeコマンドを確認してください: {error}"
            )
        })
}

#[cfg(target_os = "macos")]
fn spawn_editor(directory: &str) -> Result<(), String> {
    if Command::new("code").arg(directory).spawn().is_ok() {
        return Ok(());
    }
    Command::new("open")
        .args(["-a", "Visual Studio Code", directory])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open VS Code: {error} / VS Codeを開けません: {error}"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn spawn_editor(directory: &str) -> Result<(), String> {
    Command::new("code")
        .arg(Path::new(directory))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open VS Code: {error} / VS Codeを開けません: {error}"))
}
