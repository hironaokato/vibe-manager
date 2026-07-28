use crate::{
    discovery::discover_candidates,
    models::{
        AppSettings, DashboardSnapshot, DiscoveryCandidate, PersistedState, ProcessOrigin,
        ProjectInput, ProjectRecord, ProjectStatus, StartupPolicy,
    },
};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

const STATE_FILE: &str = "state.json";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const LOG_READ_BYTES: u64 = 768 * 1024;

#[derive(Debug, Clone)]
struct RuntimeProcess {
    pid: u32,
    generation: String,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<StateInner>,
}

struct StateInner {
    data_dir: PathBuf,
    store: Mutex<PersistedState>,
    runtimes: Mutex<HashMap<String, RuntimeProcess>>,
    discovered: Mutex<Vec<DiscoveryCandidate>>,
    quitting: AtomicBool,
}

impl AppState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("アプリデータの保存先を取得できません: {error}"))?;
        fs::create_dir_all(data_dir.join("logs"))
            .map_err(|error| format!("保存先を作成できません: {error}"))?;

        let state_path = data_dir.join(STATE_FILE);
        let store = if state_path.exists() {
            match fs::read_to_string(&state_path)
                .map_err(|error| error.to_string())
                .and_then(|contents| {
                    serde_json::from_str::<PersistedState>(&contents)
                        .map_err(|error| error.to_string())
                }) {
                Ok(store) => store,
                Err(error) => {
                    let backup = data_dir.join(format!("state.corrupt-{}.json", now_ms()));
                    let _ = fs::copy(&state_path, &backup);
                    eprintln!(
                        "Could not read saved state ({error}). A copy was preserved at {}",
                        backup.display()
                    );
                    PersistedState::default()
                }
            }
        } else {
            PersistedState::default()
        };

        let state = Self {
            inner: Arc::new(StateInner {
                data_dir,
                store: Mutex::new(store),
                runtimes: Mutex::new(HashMap::new()),
                discovered: Mutex::new(Vec::new()),
                quitting: AtomicBool::new(false),
            }),
        };
        state.reconcile_after_launch()?;
        Ok(state)
    }

    pub fn snapshot(&self) -> DashboardSnapshot {
        let store = self.lock_store();
        let discovered = self.lock_discovered();
        snapshot_from_store(&store, &discovered)
    }

    pub fn settings(&self) -> AppSettings {
        self.lock_store().settings.clone()
    }

    pub fn save_settings(
        &self,
        app: &AppHandle,
        settings: AppSettings,
    ) -> Result<DashboardSnapshot, String> {
        {
            let mut store = self.lock_store();
            store.settings = settings;
            self.persist_locked(&store)?;
        }
        self.perform_discovery(app);
        self.emit_changed(app);
        Ok(self.snapshot())
    }

    pub fn create_project(
        &self,
        app: &AppHandle,
        input: ProjectInput,
    ) -> Result<ProjectRecord, String> {
        let input = validate_input(input)?;
        let now = now_ms();
        let id = Uuid::new_v4().to_string();
        let log_path = self.inner.data_dir.join("logs").join(format!("{id}.log"));

        let external_pid = input.external_pid.filter(|pid| process_exists(*pid));
        let project = ProjectRecord {
            id,
            name: input.name,
            directory: input.directory,
            command: input.command,
            url: input.url,
            startup_policy: input.startup_policy,
            status: if external_pid.is_some() {
                ProjectStatus::Running
            } else {
                ProjectStatus::Stopped
            },
            desired_running: external_pid.is_some(),
            pid: external_pid,
            created_at: now,
            updated_at: now,
            last_started_at: external_pid.map(|_| now),
            last_stopped_at: None,
            last_exit_code: None,
            last_error: None,
            log_path: log_path.to_string_lossy().into_owned(),
            discovery_key: input.discovery_key,
            detected_port: input.detected_port,
            process_origin: if external_pid.is_some() {
                ProcessOrigin::External
            } else {
                ProcessOrigin::Manager
            },
        };

        {
            let mut store = self.lock_store();
            store.projects.push(project.clone());
            self.persist_locked(&store)?;
        }
        if external_pid.is_some() {
            let _ = write_adopted_log(&project.log_path, &project);
        }
        self.remove_discovered_for_project(&project);
        self.emit_changed(app);
        Ok(project)
    }

    pub fn update_project(
        &self,
        app: &AppHandle,
        id: &str,
        input: ProjectInput,
    ) -> Result<ProjectRecord, String> {
        let input = validate_input(input)?;
        let updated = {
            let mut store = self.lock_store();
            let project = find_project_mut(&mut store, id)?;
            if matches!(
                project.status,
                ProjectStatus::Running | ProjectStatus::Starting | ProjectStatus::Stopping
            ) {
                return Err("実行中のプロジェクトは編集できません。先に停止してください。".into());
            }
            project.name = input.name;
            project.directory = input.directory;
            project.command = input.command;
            project.url = input.url;
            project.startup_policy = input.startup_policy;
            if input.discovery_key.is_some() {
                project.discovery_key = input.discovery_key;
            }
            if input.detected_port.is_some() {
                project.detected_port = input.detected_port;
            }
            project.updated_at = now_ms();
            let updated = project.clone();
            self.persist_locked(&store)?;
            updated
        };
        self.emit_changed(app);
        Ok(updated)
    }

    pub fn delete_project(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        {
            let mut store = self.lock_store();
            let project = store
                .projects
                .iter()
                .find(|project| project.id == id)
                .ok_or_else(|| "プロジェクトが見つかりません。".to_string())?;
            if matches!(
                project.status,
                ProjectStatus::Running | ProjectStatus::Starting | ProjectStatus::Stopping
            ) {
                return Err("実行中のプロジェクトは削除できません。先に停止してください。".into());
            }
            store.projects.retain(|project| project.id != id);
            self.persist_locked(&store)?;
        }
        self.emit_changed(app);
        Ok(())
    }

    pub fn start_project(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        if self.inner.quitting.load(Ordering::SeqCst) {
            return Err("Vibe Managerを終了中です。".into());
        }

        let project = {
            let mut store = self.lock_store();
            let project = find_project_mut(&mut store, id)?;
            if let Some(pid) = project.pid {
                if process_exists(pid) {
                    project.status = ProjectStatus::Running;
                    project.desired_running = true;
                    self.persist_locked(&store)?;
                    drop(store);
                    self.emit_changed(app);
                    return Ok(());
                }
            }
            if matches!(
                project.status,
                ProjectStatus::Running | ProjectStatus::Starting
            ) {
                return Ok(());
            }
            project.status = ProjectStatus::Starting;
            project.desired_running = true;
            project.pid = None;
            project.last_error = None;
            project.updated_at = now_ms();
            let cloned = project.clone();
            self.persist_locked(&store)?;
            cloned
        };
        self.emit_changed(app);

        if let Err(error) = prepare_log(&project.log_path, &project) {
            self.mark_start_failed(app, id, &error);
            return Err(error);
        }

        let mut command = shell_command(&project.command);
        command
            .current_dir(&project.directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_child_process(&mut command);

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let message = format!("コマンドを起動できません: {error}");
                self.mark_start_failed(app, id, &message);
                return Err(message);
            }
        };

        let pid = child.id();
        let generation = Uuid::new_v4().to_string();
        let log_file = open_shared_log(&project.log_path)?;
        if let Some(stdout) = child.stdout.take() {
            pump_output(stdout, Arc::clone(&log_file), "OUT");
        }
        if let Some(stderr) = child.stderr.take() {
            pump_output(stderr, Arc::clone(&log_file), "ERR");
        }

        {
            self.lock_runtimes().insert(
                id.to_string(),
                RuntimeProcess {
                    pid,
                    generation: generation.clone(),
                },
            );
            let mut store = self.lock_store();
            let record = find_project_mut(&mut store, id)?;
            record.status = ProjectStatus::Running;
            record.desired_running = true;
            record.pid = Some(pid);
            record.process_origin = ProcessOrigin::Manager;
            record.last_started_at = Some(now_ms());
            record.last_exit_code = None;
            record.last_error = None;
            record.updated_at = now_ms();
            self.persist_locked(&store)?;
        }
        self.emit_changed(app);

        self.watch_child(app.clone(), id.to_string(), generation, child, log_file);
        Ok(())
    }

    pub fn stop_project(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        self.stop_project_with_desired(app, id, false)
    }

    fn stop_project_with_desired(
        &self,
        app: &AppHandle,
        id: &str,
        preserve_desired: bool,
    ) -> Result<(), String> {
        let pid = {
            let mut store = self.lock_store();
            let project = find_project_mut(&mut store, id)?;
            let pid = project
                .pid
                .or_else(|| self.lock_runtimes().get(id).map(|runtime| runtime.pid));
            project.status = ProjectStatus::Stopping;
            project.desired_running = preserve_desired;
            project.updated_at = now_ms();
            self.persist_locked(&store)?;
            pid
        };
        self.emit_changed(app);

        if let Some(pid) = pid {
            kill_process_tree(pid);
        }
        self.lock_runtimes().remove(id);

        {
            let mut store = self.lock_store();
            let project = find_project_mut(&mut store, id)?;
            project.status = if preserve_desired {
                ProjectStatus::RestorePending
            } else {
                ProjectStatus::Stopped
            };
            project.desired_running = preserve_desired;
            project.pid = None;
            project.last_stopped_at = Some(now_ms());
            project.updated_at = now_ms();
            self.persist_locked(&store)?;
        }
        self.emit_changed(app);
        Ok(())
    }

    pub fn restart_project(&self, app: &AppHandle, id: &str) -> Result<(), String> {
        self.stop_project_with_desired(app, id, false)?;
        self.start_project(app, id)
    }

    pub fn restore_all(&self, app: &AppHandle) -> Vec<String> {
        let ids = {
            let store = self.lock_store();
            store
                .projects
                .iter()
                .filter(|project| project.status == ProjectStatus::RestorePending)
                .map(|project| project.id.clone())
                .collect::<Vec<_>>()
        };

        ids.into_iter()
            .filter_map(|id| self.start_project(app, &id).err())
            .collect()
    }

    pub fn auto_restore(&self, app: AppHandle) {
        let ids = {
            let store = self.lock_store();
            store
                .projects
                .iter()
                .filter(|project| {
                    project.status == ProjectStatus::RestorePending
                        && project.startup_policy == StartupPolicy::Auto
                })
                .map(|project| project.id.clone())
                .collect::<Vec<_>>()
        };
        let state = self.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(600));
            for id in ids {
                if state.inner.quitting.load(Ordering::SeqCst) {
                    break;
                }
                let _ = state.start_project(&app, &id);
                thread::sleep(Duration::from_millis(180));
            }
        });
    }

    pub fn get_logs(&self, id: &str) -> Result<String, String> {
        let path = {
            let store = self.lock_store();
            store
                .projects
                .iter()
                .find(|project| project.id == id)
                .map(|project| PathBuf::from(&project.log_path))
                .ok_or_else(|| "プロジェクトが見つかりません。".to_string())?
        };
        read_log_tail(&path)
    }

    pub fn clear_logs(&self, id: &str) -> Result<(), String> {
        let path = {
            let store = self.lock_store();
            store
                .projects
                .iter()
                .find(|project| project.id == id)
                .map(|project| PathBuf::from(&project.log_path))
                .ok_or_else(|| "プロジェクトが見つかりません。".to_string())?
        };
        File::create(path)
            .map(|_| ())
            .map_err(|error| format!("ログを消去できません: {error}"))
    }

    pub fn discovery_candidate(&self, key: &str) -> Option<DiscoveryCandidate> {
        self.lock_discovered()
            .iter()
            .find(|candidate| candidate.key == key)
            .cloned()
    }

    pub fn refresh_discovery(&self, app: &AppHandle) {
        self.perform_discovery(app);
    }

    pub fn ignore_discovery_candidate(&self, app: &AppHandle, key: &str) -> Result<(), String> {
        {
            let mut store = self.lock_store();
            if !store
                .ignored_discovery_keys
                .iter()
                .any(|ignored| ignored == key)
            {
                store.ignored_discovery_keys.push(key.to_string());
                self.persist_locked(&store)?;
            }
        }
        self.lock_discovered()
            .retain(|candidate| candidate.key != key);
        self.emit_changed(app);
        Ok(())
    }

    pub fn clear_ignored_discovery_candidates(&self, app: &AppHandle) -> Result<(), String> {
        {
            let mut store = self.lock_store();
            store.ignored_discovery_keys.clear();
            self.persist_locked(&store)?;
        }
        self.perform_discovery(app);
        Ok(())
    }

    pub fn start_discovery_monitor(&self, app: AppHandle) {
        let state = self.clone();
        thread::spawn(move || {
            state.perform_discovery(&app);
            loop {
                for _ in 0..10 {
                    thread::sleep(Duration::from_secs(1));
                    if state.inner.quitting.load(Ordering::SeqCst) {
                        return;
                    }
                }
                state.perform_discovery(&app);
            }
        });
    }

    fn perform_discovery(&self, app: &AppHandle) {
        if self.inner.quitting.load(Ordering::SeqCst) {
            return;
        }
        let (settings, ignored) = {
            let store = self.lock_store();
            (
                store.settings.clone(),
                store
                    .ignored_discovery_keys
                    .iter()
                    .cloned()
                    .collect::<std::collections::HashSet<_>>(),
            )
        };

        let mut candidates = discover_candidates(&settings);
        candidates.retain(|candidate| !ignored.contains(&candidate.key));

        let mut reconciled = false;
        {
            let mut store = self.lock_store();
            for candidate in &candidates {
                let Some(project) = store
                    .projects
                    .iter_mut()
                    .find(|project| project.discovery_key.as_deref() == Some(&candidate.key))
                else {
                    continue;
                };
                if project.process_origin == ProcessOrigin::Manager
                    && matches!(
                        project.status,
                        ProjectStatus::Running | ProjectStatus::Starting | ProjectStatus::Stopping
                    )
                {
                    continue;
                }
                if project.pid != Some(candidate.pid)
                    || project.status != ProjectStatus::Running
                    || project.process_origin != ProcessOrigin::External
                {
                    project.status = ProjectStatus::Running;
                    project.desired_running = true;
                    project.pid = Some(candidate.pid);
                    project.detected_port = Some(candidate.port);
                    project.process_origin = ProcessOrigin::External;
                    project.last_started_at = Some(now_ms());
                    project.last_stopped_at = None;
                    project.last_exit_code = None;
                    project.last_error = None;
                    project.updated_at = now_ms();
                    reconciled = true;
                }
            }
            if reconciled {
                let _ = self.persist_locked(&store);
            }
            let registered_keys = store
                .projects
                .iter()
                .filter_map(|project| project.discovery_key.as_deref())
                .collect::<std::collections::HashSet<_>>();
            let registered_pids = store
                .projects
                .iter()
                .filter_map(|project| project.pid)
                .collect::<std::collections::HashSet<_>>();
            candidates.retain(|candidate| {
                !registered_keys.contains(candidate.key.as_str())
                    && !registered_pids.contains(&candidate.pid)
            });
        }

        let previous_discovered_at = self
            .lock_discovered()
            .iter()
            .map(|candidate| (candidate.key.clone(), candidate.discovered_at))
            .collect::<HashMap<_, _>>();
        for candidate in &mut candidates {
            if let Some(discovered_at) = previous_discovered_at.get(&candidate.key) {
                candidate.discovered_at = *discovered_at;
            }
        }

        let mut adopted = Vec::new();
        if settings.auto_register_discovered {
            let auto_candidates = candidates
                .iter()
                .filter(|candidate| {
                    candidate.confidence >= 75
                        && !candidate.directory.is_empty()
                        && !candidate.command.is_empty()
                        && Path::new(&candidate.directory).is_dir()
                })
                .cloned()
                .collect::<Vec<_>>();
            if !auto_candidates.is_empty() {
                let mut store = self.lock_store();
                for candidate in auto_candidates {
                    if store
                        .projects
                        .iter()
                        .any(|project| project.discovery_key.as_deref() == Some(&candidate.key))
                    {
                        continue;
                    }
                    let project = self.project_from_candidate(
                        &candidate,
                        settings.default_startup_policy.clone(),
                    );
                    candidates.retain(|item| item.key != candidate.key);
                    store.projects.push(project.clone());
                    adopted.push(project);
                }
                if !adopted.is_empty() {
                    let _ = self.persist_locked(&store);
                }
            }
        }

        for project in &adopted {
            let _ = write_adopted_log(&project.log_path, project);
        }
        let changed = {
            let mut discovered = self.lock_discovered();
            if *discovered == candidates {
                false
            } else {
                *discovered = candidates;
                true
            }
        };
        if changed || reconciled || !adopted.is_empty() {
            self.emit_changed(app);
        }
    }

    fn project_from_candidate(
        &self,
        candidate: &DiscoveryCandidate,
        startup_policy: StartupPolicy,
    ) -> ProjectRecord {
        let id = Uuid::new_v4().to_string();
        let now = now_ms();
        ProjectRecord {
            id: id.clone(),
            name: candidate.name.clone(),
            directory: candidate.directory.clone(),
            command: candidate.command.clone(),
            url: Some(candidate.url.clone()),
            startup_policy,
            status: ProjectStatus::Running,
            desired_running: true,
            pid: Some(candidate.pid),
            created_at: now,
            updated_at: now,
            last_started_at: Some(now),
            last_stopped_at: None,
            last_exit_code: None,
            last_error: None,
            log_path: self
                .inner
                .data_dir
                .join("logs")
                .join(format!("{id}.log"))
                .to_string_lossy()
                .into_owned(),
            discovery_key: Some(candidate.key.clone()),
            detected_port: Some(candidate.port),
            process_origin: ProcessOrigin::External,
        }
    }

    fn remove_discovered_for_project(&self, project: &ProjectRecord) {
        self.lock_discovered().retain(|candidate| {
            candidate.pid != project.pid.unwrap_or_default()
                && project.discovery_key.as_deref() != Some(&candidate.key)
        });
    }

    pub fn prepare_for_exit(&self, app: &AppHandle) {
        if self.inner.quitting.swap(true, Ordering::SeqCst) {
            return;
        }
        let active = {
            let mut store = self.lock_store();
            let active = store
                .projects
                .iter()
                .filter_map(|project| {
                    if matches!(
                        project.status,
                        ProjectStatus::Running | ProjectStatus::Starting | ProjectStatus::Stopping
                    ) {
                        project.pid.map(|pid| (project.id.clone(), pid))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            for project in &mut store.projects {
                if matches!(
                    project.status,
                    ProjectStatus::Running | ProjectStatus::Starting | ProjectStatus::Stopping
                ) {
                    project.status = if project.desired_running {
                        ProjectStatus::RestorePending
                    } else {
                        ProjectStatus::Stopped
                    };
                    project.pid = None;
                    project.updated_at = now_ms();
                }
            }
            let _ = self.persist_locked(&store);
            active
        };
        self.emit_changed(app);

        for (_, pid) in &active {
            kill_process_tree(*pid);
        }
        self.lock_runtimes().clear();
    }

    pub fn start_monitor(&self, app: AppHandle) {
        let state = self.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(3));
            if state.inner.quitting.load(Ordering::SeqCst) {
                break;
            }
            let runtime_ids = state.lock_runtimes().keys().cloned().collect::<Vec<_>>();
            let detached = {
                let store = state.lock_store();
                store
                    .projects
                    .iter()
                    .filter_map(|project| {
                        if project.status == ProjectStatus::Running
                            && !runtime_ids.contains(&project.id)
                        {
                            project.pid.map(|pid| (project.id.clone(), pid))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            };
            for (id, pid) in detached {
                if !process_exists(pid) {
                    state.mark_unexpected_exit(&app, &id, None, "プロセスが見つかりません。");
                }
            }
        });
    }

    fn watch_child(
        &self,
        app: AppHandle,
        id: String,
        generation: String,
        mut child: Child,
        log_file: Arc<Mutex<File>>,
    ) {
        let state = self.clone();
        thread::spawn(move || {
            let exit = child.wait();
            let _ = writeln!(
                lock_file(&log_file),
                "\n[{}] --- Process exited: {:?} ---",
                timestamp_label(),
                exit.as_ref().ok().and_then(|status| status.code())
            );

            let is_current = state
                .lock_runtimes()
                .get(&id)
                .map(|runtime| runtime.generation == generation)
                .unwrap_or(false);
            if !is_current {
                return;
            }
            state.lock_runtimes().remove(&id);
            if state.inner.quitting.load(Ordering::SeqCst) {
                return;
            }

            match exit {
                Ok(status) => state.mark_unexpected_exit(
                    &app,
                    &id,
                    status.code(),
                    if status.success() {
                        "プロセスが終了しました。"
                    } else {
                        "プロセスがエラー終了しました。"
                    },
                ),
                Err(error) => state.mark_unexpected_exit(
                    &app,
                    &id,
                    None,
                    &format!("終了状態を取得できません: {error}"),
                ),
            }
        });
    }

    fn mark_start_failed(&self, app: &AppHandle, id: &str, message: &str) {
        {
            let mut store = self.lock_store();
            if let Ok(project) = find_project_mut(&mut store, id) {
                project.status = ProjectStatus::Crashed;
                project.desired_running = true;
                project.pid = None;
                project.last_error = Some(message.to_string());
                project.updated_at = now_ms();
                let _ = self.persist_locked(&store);
            }
        }
        self.emit_changed(app);
    }

    fn mark_unexpected_exit(
        &self,
        app: &AppHandle,
        id: &str,
        exit_code: Option<i32>,
        message: &str,
    ) {
        {
            let mut store = self.lock_store();
            if let Ok(project) = find_project_mut(&mut store, id) {
                if matches!(
                    project.status,
                    ProjectStatus::Stopping | ProjectStatus::Stopped
                ) {
                    return;
                }
                project.status = ProjectStatus::Crashed;
                project.desired_running = true;
                project.pid = None;
                project.last_exit_code = exit_code;
                project.last_error = Some(message.to_string());
                project.last_stopped_at = Some(now_ms());
                project.updated_at = now_ms();
                let _ = self.persist_locked(&store);
            }
        }
        self.emit_changed(app);
    }

    fn reconcile_after_launch(&self) -> Result<(), String> {
        let mut store = self.lock_store();
        for project in &mut store.projects {
            let process_is_alive = project.pid.map(process_exists).unwrap_or(false);
            if process_is_alive {
                project.status = ProjectStatus::Running;
                project.desired_running = true;
            } else if matches!(
                project.status,
                ProjectStatus::Running | ProjectStatus::Starting | ProjectStatus::Stopping
            ) {
                project.pid = None;
                project.status = if project.desired_running {
                    ProjectStatus::RestorePending
                } else {
                    ProjectStatus::Stopped
                };
            }
        }
        self.persist_locked(&store)
    }

    fn persist_locked(&self, store: &PersistedState) -> Result<(), String> {
        let contents = serde_json::to_string_pretty(store)
            .map_err(|error| format!("状態を保存用に変換できません: {error}"))?;
        fs::write(self.inner.data_dir.join(STATE_FILE), contents)
            .map_err(|error| format!("状態を保存できません: {error}"))
    }

    fn emit_changed(&self, app: &AppHandle) {
        let snapshot = self.snapshot();
        let tooltip = if snapshot.crashed_count > 0 {
            format!("Vibe Manager • {}件が異常終了", snapshot.crashed_count)
        } else if snapshot.restore_count > 0 {
            format!("Vibe Manager • {}件が復元待ち", snapshot.restore_count)
        } else if snapshot.discovery_count > 0 {
            format!(
                "Vibe Manager • {}件のローカルサーバーを検出",
                snapshot.discovery_count
            )
        } else {
            format!("Vibe Manager • {}件が起動中", snapshot.running_count)
        };
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(tooltip));
        }
        let _ = app.emit("manager-state-changed", snapshot);
    }

    fn lock_store(&self) -> MutexGuard<'_, PersistedState> {
        self.inner
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_runtimes(&self) -> MutexGuard<'_, HashMap<String, RuntimeProcess>> {
        self.inner
            .runtimes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_discovered(&self) -> MutexGuard<'_, Vec<DiscoveryCandidate>> {
        self.inner
            .discovered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn snapshot_from_store(
    store: &PersistedState,
    discovered: &[DiscoveryCandidate],
) -> DashboardSnapshot {
    DashboardSnapshot {
        settings: store.settings.clone(),
        projects: store.projects.clone(),
        restore_count: store
            .projects
            .iter()
            .filter(|project| project.status == ProjectStatus::RestorePending)
            .count(),
        running_count: store
            .projects
            .iter()
            .filter(|project| project.status == ProjectStatus::Running)
            .count(),
        crashed_count: store
            .projects
            .iter()
            .filter(|project| project.status == ProjectStatus::Crashed)
            .count(),
        discovery_candidates: discovered.to_vec(),
        discovery_count: discovered.len(),
        ignored_discovery_count: store.ignored_discovery_keys.len(),
    }
}

fn find_project_mut<'a>(
    store: &'a mut PersistedState,
    id: &str,
) -> Result<&'a mut ProjectRecord, String> {
    store
        .projects
        .iter_mut()
        .find(|project| project.id == id)
        .ok_or_else(|| "プロジェクトが見つかりません。".to_string())
}

fn validate_input(mut input: ProjectInput) -> Result<ProjectInput, String> {
    input.name = input.name.trim().to_string();
    input.directory = input.directory.trim().to_string();
    input.command = input.command.trim().to_string();
    input.url = input
        .url
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty());

    if input.name.is_empty() {
        return Err("プロジェクト名を入力してください。".into());
    }
    if input.command.is_empty() {
        return Err("起動コマンドを入力してください。".into());
    }
    let directory = Path::new(&input.directory);
    if !directory.exists() {
        return Err("指定した作業ディレクトリが見つかりません。".into());
    }
    if !directory.is_dir() {
        return Err("作業ディレクトリにはフォルダーを指定してください。".into());
    }
    if let Some(url) = &input.url {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("URLはhttp://またはhttps://から入力してください。".into());
        }
    }
    Ok(input)
}

fn prepare_log(path: &str, project: &ProjectRecord) -> Result<(), String> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("ログフォルダーを作成できません: {error}"))?;
    }
    if path.metadata().map(|meta| meta.len()).unwrap_or(0) > MAX_LOG_BYTES {
        let rotated = path.with_extension("previous.log");
        let _ = fs::remove_file(&rotated);
        fs::rename(path, rotated)
            .map_err(|error| format!("ログをローテーションできません: {error}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("ログを開けません: {error}"))?;
    writeln!(
        file,
        "\n[{}] --- Starting {} ---\n[CMD] {}\n[CWD] {}",
        timestamp_label(),
        project.name,
        project.command,
        project.directory
    )
    .map_err(|error| format!("ログを書き込めません: {error}"))
}

fn write_adopted_log(path: &str, project: &ProjectRecord) -> Result<(), String> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("ログフォルダーを作成できません: {error}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("ログを開けません: {error}"))?;
    writeln!(
        file,
        "[{}] --- Adopted external process {} (PID {}) ---\n\
         [VIBE] 取り込み前のログは取得できません。次回Vibe Managerから起動すると記録されます。",
        timestamp_label(),
        project.name,
        project.pid.unwrap_or_default()
    )
    .map_err(|error| format!("ログを書き込めません: {error}"))
}

fn open_shared_log(path: &str) -> Result<Arc<Mutex<File>>, String> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|file| Arc::new(Mutex::new(file)))
        .map_err(|error| format!("ログを開けません: {error}"))
}

fn pump_output<R: Read + Send + 'static>(
    reader: R,
    log_file: Arc<Mutex<File>>,
    stream_name: &'static str,
) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            match line {
                Ok(line) => {
                    let _ = writeln!(lock_file(&log_file), "[{stream_name}] {line}");
                }
                Err(error) => {
                    let _ = writeln!(
                        lock_file(&log_file),
                        "[VIBE] Could not read {stream_name}: {error}"
                    );
                    break;
                }
            }
        }
    });
}

fn lock_file(file: &Arc<Mutex<File>>) -> MutexGuard<'_, File> {
    file.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn read_log_tail(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Ok("まだログはありません。".into());
    }
    let mut file = File::open(path).map_err(|error| format!("ログを開けません: {error}"))?;
    let len = file
        .metadata()
        .map_err(|error| format!("ログ情報を取得できません: {error}"))?
        .len();
    let start = len.saturating_sub(LOG_READ_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("ログを読み取れません: {error}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("ログを読み取れません: {error}"))?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if start > 0 {
        if let Some(first_newline) = text.find('\n') {
            text = format!("…（先頭部分を省略）\n{}", &text[first_newline + 1..]);
        }
    }
    Ok(text)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("unix:{seconds}")
}

#[cfg(windows)]
fn shell_command(script: &str) -> Command {
    let mut command = Command::new("cmd.exe");
    command.args(["/D", "/S", "/C", script]);
    command
}

#[cfg(not(windows))]
fn shell_command(script: &str) -> Command {
    let mut command = Command::new("/bin/zsh");
    command.args(["-lc", script]);
    command
}

#[cfg(windows)]
fn configure_child_process(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(unix)]
fn configure_child_process(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new("tasklist.exe");
    command
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW);
    command
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains(&format!(",\"{pid}\",")))
        .unwrap_or(false)
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as i32, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn kill_process_tree(pid: u32) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = Command::new("taskkill.exe")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(unix)]
fn kill_process_tree(pid: u32) {
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    thread::sleep(Duration::from_millis(450));
    if process_exists(pid) {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input(directory: &Path) -> ProjectInput {
        ProjectInput {
            name: " API ".into(),
            directory: directory.to_string_lossy().into_owned(),
            command: " npm run dev ".into(),
            url: Some(" http://localhost:3000 ".into()),
            startup_policy: StartupPolicy::Ask,
            discovery_key: None,
            detected_port: None,
            external_pid: None,
        }
    }

    #[test]
    fn input_validation_trims_values() {
        let temp = tempfile::tempdir().unwrap();
        let validated = validate_input(valid_input(temp.path())).unwrap();
        assert_eq!(validated.name, "API");
        assert_eq!(validated.command, "npm run dev");
        assert_eq!(validated.url.as_deref(), Some("http://localhost:3000"));
    }

    #[test]
    fn input_validation_rejects_missing_directory() {
        let mut input = valid_input(Path::new("definitely-missing-directory"));
        input.name = "Project".into();
        assert!(validate_input(input).is_err());
    }

    #[test]
    fn snapshot_counts_statuses() {
        let mut state = PersistedState::default();
        let temp = tempfile::tempdir().unwrap();
        for status in [
            ProjectStatus::Running,
            ProjectStatus::RestorePending,
            ProjectStatus::Crashed,
        ] {
            let input = validate_input(valid_input(temp.path())).unwrap();
            state.projects.push(ProjectRecord {
                id: Uuid::new_v4().to_string(),
                name: input.name,
                directory: input.directory,
                command: input.command,
                url: input.url,
                startup_policy: input.startup_policy,
                status,
                desired_running: true,
                pid: None,
                created_at: 0,
                updated_at: 0,
                last_started_at: None,
                last_stopped_at: None,
                last_exit_code: None,
                last_error: None,
                log_path: String::new(),
                discovery_key: None,
                detected_port: None,
                process_origin: ProcessOrigin::Manager,
            });
        }
        let snapshot = snapshot_from_store(&state, &[]);
        assert_eq!(snapshot.running_count, 1);
        assert_eq!(snapshot.restore_count, 1);
        assert_eq!(snapshot.crashed_count, 1);
    }

    #[test]
    fn managed_shell_process_can_be_started_and_stopped() {
        #[cfg(windows)]
        let script = "ping 127.0.0.1 -n 30 > nul";
        #[cfg(not(windows))]
        let script = "sleep 30";

        let mut command = shell_command(script);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        configure_child_process(&mut command);
        let mut child = command.spawn().expect("test process should start");
        let pid = child.id();
        thread::sleep(Duration::from_millis(150));
        assert!(process_exists(pid));

        kill_process_tree(pid);
        let _ = child.wait();
        assert!(!process_exists(pid));
    }

    #[test]
    fn log_reader_returns_the_tail_of_a_large_log() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("large.log");
        let contents = format!(
            "old line\n{}\nlast line",
            "x".repeat(LOG_READ_BYTES as usize)
        );
        fs::write(&path, contents).unwrap();
        let tail = read_log_tail(&path).unwrap();
        assert!(tail.starts_with("…（先頭部分を省略）"));
        assert!(tail.ends_with("last line"));
    }
}
