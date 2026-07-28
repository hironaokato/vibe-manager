use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StartupPolicy {
    Auto,
    Ask,
    Manual,
}

impl Default for StartupPolicy {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Crashed,
    RestorePending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProcessOrigin {
    Manager,
    External,
}

impl Default for ProcessOrigin {
    fn default() -> Self {
        Self::Manager
    }
}

impl Default for ProjectStatus {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub onboarding_complete: bool,
    pub launch_at_login: bool,
    pub default_startup_policy: StartupPolicy,
    pub discovery_enabled: bool,
    pub auto_register_discovered: bool,
    pub workspace_roots: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            onboarding_complete: false,
            launch_at_login: false,
            default_startup_policy: StartupPolicy::Ask,
            discovery_enabled: true,
            auto_register_discovered: false,
            workspace_roots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub directory: String,
    pub command: String,
    pub url: Option<String>,
    pub startup_policy: StartupPolicy,
    pub status: ProjectStatus,
    pub desired_running: bool,
    pub pid: Option<u32>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_started_at: Option<i64>,
    pub last_stopped_at: Option<i64>,
    pub last_exit_code: Option<i32>,
    pub last_error: Option<String>,
    pub log_path: String,
    #[serde(default)]
    pub discovery_key: Option<String>,
    #[serde(default)]
    pub detected_port: Option<u16>,
    #[serde(default)]
    pub process_origin: ProcessOrigin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistedState {
    pub schema_version: u32,
    pub settings: AppSettings,
    pub projects: Vec<ProjectRecord>,
    pub ignored_discovery_keys: Vec<String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            settings: AppSettings::default(),
            projects: Vec::new(),
            ignored_discovery_keys: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCandidate {
    pub key: String,
    pub pid: u32,
    pub port: u16,
    pub address: String,
    pub url: String,
    pub name: String,
    pub process_name: String,
    pub process_type: String,
    pub executable: String,
    pub command: String,
    pub directory: String,
    pub external_exposure: bool,
    pub confidence: u8,
    pub discovered_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub settings: AppSettings,
    pub projects: Vec<ProjectRecord>,
    pub restore_count: usize,
    pub running_count: usize,
    pub crashed_count: usize,
    pub discovery_candidates: Vec<DiscoveryCandidate>,
    pub discovery_count: usize,
    pub ignored_discovery_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub name: String,
    pub directory: String,
    pub command: String,
    pub url: Option<String>,
    pub startup_policy: StartupPolicy,
    #[serde(default)]
    pub discovery_key: Option<String>,
    #[serde(default)]
    pub detected_port: Option<u16>,
    #[serde(default)]
    pub external_pid: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    pub onboarding_complete: bool,
    pub launch_at_login: bool,
    pub default_startup_policy: StartupPolicy,
    pub discovery_enabled: bool,
    pub auto_register_discovered: bool,
    pub workspace_roots: Vec<String>,
}

impl From<SettingsInput> for AppSettings {
    fn from(value: SettingsInput) -> Self {
        Self {
            onboarding_complete: value.onboarding_complete,
            launch_at_login: value.launch_at_login,
            default_startup_policy: value.default_startup_policy,
            discovery_enabled: value.discovery_enabled,
            auto_register_discovered: value.auto_register_discovered,
            workspace_roots: value.workspace_roots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_version_one_state_with_discovery_defaults() {
        let state: PersistedState = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "settings": {
                "onboardingComplete": true,
                "launchAtLogin": true,
                "defaultStartupPolicy": "ask"
            },
            "projects": [{
                "id": "old-project",
                "name": "Existing app",
                "directory": "C:\\Projects\\existing",
                "command": "npm run dev",
                "url": "http://localhost:3000",
                "startupPolicy": "ask",
                "status": "stopped",
                "desiredRunning": false,
                "pid": null,
                "createdAt": 1,
                "updatedAt": 1,
                "lastStartedAt": null,
                "lastStoppedAt": null,
                "lastExitCode": null,
                "lastError": null,
                "logPath": "existing.log"
            }]
        }))
        .expect("version one state should remain readable");

        assert!(state.settings.discovery_enabled);
        assert!(!state.settings.auto_register_discovered);
        assert!(state.settings.workspace_roots.is_empty());
        assert!(state.ignored_discovery_keys.is_empty());
        assert_eq!(state.projects[0].process_origin, ProcessOrigin::Manager);
        assert!(state.projects[0].discovery_key.is_none());
    }
}
