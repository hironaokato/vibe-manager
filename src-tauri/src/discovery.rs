use crate::models::{AppSettings, DiscoveryCandidate};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::{Pid, System};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ListeningSocket {
    pid: u32,
    port: u16,
    address: String,
}

pub fn discover_candidates(settings: &AppSettings) -> Vec<DiscoveryCandidate> {
    if !settings.discovery_enabled {
        return Vec::new();
    }

    let sockets = listening_sockets();
    if sockets.is_empty() {
        return Vec::new();
    }

    let system = System::new_all();
    let own_pid = std::process::id();
    let roots = settings
        .workspace_roots
        .iter()
        .filter_map(|root| normalize_existing_path(root))
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for socket in sockets {
        if socket.pid == 0 || socket.pid == own_pid || !seen.insert((socket.pid, socket.port)) {
            continue;
        }
        let Some(process) = system.process(Pid::from_u32(socket.pid)) else {
            continue;
        };

        let process_name = process.name().to_string_lossy().into_owned();
        let process_name_lower = process_name.to_ascii_lowercase();
        if is_excluded_process(&process_name_lower) {
            continue;
        }

        let (process_type, runtime_score) = classify_runtime(&process_name_lower);
        let directory = best_directory(&system, process.pid())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        let directory_path = (!directory.is_empty()).then(|| PathBuf::from(&directory));
        let under_root = directory_path
            .as_deref()
            .map(|path| roots.iter().any(|root| path_starts_with(path, root)))
            .unwrap_or(false);
        let common_port = is_common_dev_port(socket.port);

        if !roots.is_empty() && !under_root {
            continue;
        }
        if roots.is_empty() && runtime_score == 0 && !common_port {
            continue;
        }

        let command = best_command(&system, process.pid());
        let executable = process
            .exe()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = directory_path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{process_type} :{}", socket.port));
        let mut confidence = runtime_score;
        if directory_path.is_some() {
            confidence += 25;
        }
        if under_root {
            confidence += 25;
        }
        if common_port {
            confidence += 15;
        }
        if !command.is_empty() {
            confidence += 10;
        }

        let stable_identity = if !directory.is_empty() {
            directory.to_ascii_lowercase()
        } else if !executable.is_empty() {
            executable.to_ascii_lowercase()
        } else {
            process_name_lower.clone()
        };
        candidates.push(DiscoveryCandidate {
            key: format!("{stable_identity}|{}|{}", process_name_lower, socket.port),
            pid: socket.pid,
            port: socket.port,
            address: socket.address.clone(),
            url: format!("http://localhost:{}", socket.port),
            name,
            process_name,
            process_type,
            executable,
            command,
            directory,
            external_exposure: is_external_address(&socket.address),
            confidence: confidence.min(100),
            discovered_at: now_ms(),
        });
    }

    candidates.sort_by(|left, right| {
        right
            .confidence
            .cmp(&left.confidence)
            .then_with(|| left.port.cmp(&right.port))
    });
    candidates
}

fn best_directory(system: &System, pid: Pid) -> Option<PathBuf> {
    let mut current = Some(pid);
    for _ in 0..4 {
        let process = system.process(current?)?;
        if let Some(cwd) = process.cwd().filter(|path| !path.as_os_str().is_empty()) {
            return Some(cwd.to_path_buf());
        }
        current = process.parent();
    }
    None
}

fn best_command(system: &System, pid: Pid) -> String {
    let mut current = Some(pid);
    let mut fallback = String::new();
    for _ in 0..4 {
        let Some(process) = current.and_then(|pid| system.process(pid)) else {
            break;
        };
        let command = join_command(process.cmd());
        if fallback.is_empty() && !command.is_empty() {
            fallback = command.clone();
        }
        let lower = command.to_ascii_lowercase();
        if ["npm", "pnpm", "yarn", "bun run", "deno", "cargo run"]
            .iter()
            .any(|marker| lower.contains(marker))
        {
            return command;
        }
        current = process.parent();
    }
    fallback
}

fn join_command(arguments: &[std::ffi::OsString]) -> String {
    arguments
        .iter()
        .map(|argument| shell_quote(&argument.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(windows)]
fn shell_quote(argument: &str) -> String {
    if argument.contains([' ', '\t', '"']) {
        format!("\"{}\"", argument.replace('"', "\\\""))
    } else {
        argument.to_string()
    }
}

#[cfg(not(windows))]
fn shell_quote(argument: &str) -> String {
    if argument
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_./:".contains(character))
    {
        argument.to_string()
    } else {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
}

fn classify_runtime(process_name: &str) -> (String, u8) {
    if process_name.contains("node") {
        ("Node.js".into(), 40)
    } else if process_name.contains("bun") {
        ("Bun".into(), 40)
    } else if process_name.contains("deno") {
        ("Deno".into(), 40)
    } else if process_name.contains("python") || process_name.contains("uvicorn") {
        ("Python".into(), 40)
    } else if process_name.contains("cargo") || process_name.contains("rust") {
        ("Rust".into(), 35)
    } else if process_name.contains("dotnet") {
        (".NET".into(), 35)
    } else if process_name.contains("java") {
        ("Java".into(), 30)
    } else if process_name.contains("php") {
        ("PHP".into(), 35)
    } else if process_name.contains("ruby") {
        ("Ruby".into(), 35)
    } else {
        ("Local server / ローカルサーバー".into(), 0)
    }
}

fn is_excluded_process(process_name: &str) -> bool {
    [
        "system",
        "svchost",
        "lsass",
        "services",
        "postgres",
        "mysqld",
        "mariadbd",
        "redis-server",
        "docker",
        "com.docker",
        "vibe-manager",
    ]
    .iter()
    .any(|name| process_name == *name || process_name.starts_with(&format!("{name}.")))
}

fn is_common_dev_port(port: u16) -> bool {
    matches!(
        port,
        3000..=3010
            | 4000..=4010
            | 4200
            | 4321
            | 5000..=5010
            | 5173..=5180
            | 8000..=8010
            | 8080..=8090
            | 8787..=8790
            | 9000..=9010
    )
}

fn is_external_address(address: &str) -> bool {
    matches!(address, "0.0.0.0" | "::" | "*" | "[::]")
}

fn normalize_existing_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path.trim());
    if !path.is_dir() {
        return None;
    }
    path.canonicalize()
        .ok()
        .or_else(|| Some(path.to_path_buf()))
}

fn path_starts_with(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        PathBuf::from(path.to_string_lossy().to_ascii_lowercase())
            .starts_with(PathBuf::from(root.to_string_lossy().to_ascii_lowercase()))
    }
    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
}

#[cfg(windows)]
fn listening_sockets() -> Vec<ListeningSocket> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = "$ErrorActionPreference='SilentlyContinue'; @(\n\
        Get-NetTCPConnection -State Listen | ForEach-Object {\n\
          [PSCustomObject]@{ address=$_.LocalAddress; port=$_.LocalPort; pid=$_.OwningProcess }\n\
        }\n\
      ) | ConvertTo-Json -Compress";
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            SCRIPT,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    parse_windows_listener_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(windows)]
fn parse_windows_listener_json(contents: &str) -> Vec<ListeningSocket> {
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(contents.trim()) else {
        return Vec::new();
    };
    values
        .into_iter()
        .filter_map(|value| {
            Some(ListeningSocket {
                pid: value.get("pid")?.as_u64()? as u32,
                port: value.get("port")?.as_u64()? as u16,
                address: value.get("address")?.as_str()?.to_string(),
            })
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn listening_sockets() -> Vec<ListeningSocket> {
    let output = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    parse_lsof_listeners(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn listening_sockets() -> Vec<ListeningSocket> {
    Vec::new()
}

#[cfg(any(test, target_os = "macos"))]
fn parse_lsof_listeners(contents: &str) -> Vec<ListeningSocket> {
    let mut pid = None;
    let mut listeners = Vec::new();
    for line in contents.lines() {
        let Some((prefix, value)) = line.split_at_checked(1) else {
            continue;
        };
        match prefix {
            "p" => pid = value.parse::<u32>().ok(),
            "n" => {
                let Some(port) = value.rsplit(':').next().and_then(|port| port.parse().ok()) else {
                    continue;
                };
                let address = value
                    .rsplit_once(':')
                    .map(|(address, _)| address.trim_matches(['[', ']']).to_string())
                    .unwrap_or_default();
                if let Some(pid) = pid {
                    listeners.push(ListeningSocket { pid, port, address });
                }
            }
            _ => {}
        }
    }
    listeners
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lsof_machine_output() {
        let parsed = parse_lsof_listeners("p123\ncnode\nn127.0.0.1:3000\nn*:5173\n");
        assert_eq!(
            parsed,
            vec![
                ListeningSocket {
                    pid: 123,
                    port: 3000,
                    address: "127.0.0.1".into(),
                },
                ListeningSocket {
                    pid: 123,
                    port: 5173,
                    address: "*".into(),
                }
            ]
        );
    }

    #[test]
    fn classifies_common_development_runtimes() {
        assert_eq!(classify_runtime("node.exe").0, "Node.js");
        assert_eq!(classify_runtime("python3").0, "Python");
        assert_eq!(classify_runtime("unknown").1, 0);
    }

    #[cfg(windows)]
    #[test]
    fn detects_a_real_windows_listener() {
        use std::{net::TcpListener, thread, time::Duration};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::sleep(Duration::from_millis(150));
        let sockets = listening_sockets();
        assert!(sockets
            .iter()
            .any(|socket| socket.pid == std::process::id() && socket.port == port));
    }

    #[cfg(windows)]
    #[test]
    fn discovers_a_child_node_server_as_a_candidate() {
        use std::{net::TcpListener, process::Stdio, thread, time::Duration};

        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let directory = std::env::current_dir().unwrap();
        let script =
            format!("require('http').createServer((_,r)=>r.end('ok')).listen({port},'127.0.0.1')");
        let mut child = Command::new("node")
            .args(["-e", &script])
            .current_dir(&directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Node.js is required by the project test environment");

        let settings = AppSettings {
            workspace_roots: vec![directory.to_string_lossy().into_owned()],
            ..AppSettings::default()
        };
        let mut found = None;
        for _ in 0..6 {
            thread::sleep(Duration::from_millis(250));
            let candidates = discover_candidates(&settings);
            found = candidates
                .into_iter()
                .find(|candidate| candidate.pid == child.id() && candidate.port == port);
            if found.is_some() {
                break;
            }
        }
        let _ = child.kill();
        let _ = child.wait();

        let candidate = found.expect("the Node.js listener should be discovered");
        assert_eq!(candidate.process_type, "Node.js");
        assert_eq!(candidate.url, format!("http://localhost:{port}"));
        assert!(!candidate.command.is_empty());
    }
}
