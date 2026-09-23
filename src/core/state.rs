use crate::core::config::Settings;
use crate::core::error::Result;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub active: bool,
    pub daemon_running: bool,
    pub state: String, // "idle", "wakeword", "recording", "processing", "speaking"
    pub mic_active: bool, // True strictly when state is "recording"
    pub wakeword_enabled: bool,
    pub cli_tool: String,
    pub last_transcript: String,
    pub last_reply: String,
    pub updated_at: f64,
    #[serde(default)]
    pub is_busy: bool,
    #[serde(default)]
    pub current_task: String,
}

impl Default for Status {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        Self {
            active: false,
            daemon_running: false,
            state: "idle".to_string(),
            mic_active: false,
            wakeword_enabled: true,
            cli_tool: "agy".to_string(),
            last_transcript: String::new(),
            last_reply: String::new(),
            updated_at: now,
            is_busy: false,
            current_task: String::new(),
        }
    }
}

pub fn get_runtime_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(xdg);
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::from("/tmp")
}

pub fn state_json_path() -> PathBuf {
    get_runtime_dir().join("jarvis-status.json")
}

pub fn pid_file_path() -> PathBuf {
    let uid = nix::unistd::getuid().as_raw();
    get_runtime_dir().join(format!("jarvis-{}.pid", uid))
}

pub fn daemon_pid_file_path() -> PathBuf {
    let uid = nix::unistd::getuid().as_raw();
    get_runtime_dir().join(format!("jarvis-daemon-{}.pid", uid))
}

pub fn wakeword_lock_path() -> PathBuf {
    let uid = nix::unistd::getuid().as_raw();
    get_runtime_dir().join(format!("jarvis-wakeword-{}.state", uid))
}

pub fn read_status() -> Status {
    let path = state_json_path();
    if path.is_file() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(status) = serde_json::from_str::<Status>(&content) {
                return status;
            }
        }
    }
    Status::default()
}

pub fn write_status_atomic(status: &Status) -> Result<()> {
    let target = state_json_path();
    let tmp = target.with_extension("tmp");
    let json = serde_json::to_string_pretty(status)?;
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &target)?;
    Ok(())
}

pub fn update_status<F>(f: F) -> Status
where
    F: FnOnce(&mut Status),
{
    let mut current = read_status();
    f(&mut current);

    // Enforce invariant: mic_active is true strictly if state is recording
    current.mic_active = current.state == "recording";
    current.updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    if let Err(e) = write_status_atomic(&current) {
        error!("Failed to write state atomically: {e}");
    }
    current
}

pub fn set_recording(prompt_hint: &str) {
    update_status(|s| {
        s.active = true;
        s.state = "recording".to_string();
        s.mic_active = true;
        if !prompt_hint.is_empty() {
            s.last_transcript = prompt_hint.to_string();
        }
    });
}

pub fn set_processing(transcript: &str) {
    update_status(|s| {
        s.active = true;
        s.is_busy = true;
        s.state = "processing".to_string();
        s.mic_active = false;
        if !transcript.is_empty() {
            s.last_transcript = transcript.to_string();
            s.current_task = transcript.to_string();
        }
    });
}

pub fn set_busy(task: &str) {
    update_status(|s| {
        s.active = true;
        s.is_busy = true;
        s.state = "processing".to_string();
        s.mic_active = false;
        if !task.is_empty() {
            s.current_task = task.to_string();
        }
    });
}

pub fn clear_busy() {
    update_status(|s| {
        s.is_busy = false;
        s.current_task.clear();
        if s.state == "processing" {
            s.state = if s.daemon_running {
                "wakeword".to_string()
            } else {
                "idle".to_string()
            };
            s.active = false;
        }
    });
}

pub fn ensure_idle() {
    update_status(|s| {
        s.is_busy = false;
        s.active = false;
        s.current_task.clear();
        s.mic_active = false;
        s.state = if s.daemon_running {
            "wakeword".to_string()
        } else {
            "idle".to_string()
        };
    });
}

pub fn set_speaking(reply: &str) {
    update_status(|s| {
        s.active = true;
        s.is_busy = false;
        s.current_task.clear();
        s.state = "speaking".to_string();
        s.mic_active = false;
        if !reply.is_empty() {
            s.last_reply = reply.to_string();
        }
    });
}

pub fn set_idle(wakeword_active: bool) {
    update_status(|s| {
        s.active = false;
        s.is_busy = false;
        s.current_task.clear();
        s.mic_active = false;
        s.state = if wakeword_active {
            "wakeword".to_string()
        } else {
            "idle".to_string()
        };
    });
}

pub fn sync_with_settings(settings: &Settings) {
    update_status(|s| {
        s.wakeword_enabled = settings.wakeword_enabled;
        s.cli_tool = settings.cli_ai_tool.clone();
    });
}

pub fn write_pid(path: &Path) -> Result<()> {
    let pid = std::process::id();
    fs::write(path, pid.to_string())?;
    Ok(())
}

pub fn read_pid(path: &Path) -> Option<i32> {
    if path.is_file() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                return Some(pid);
            }
        }
    }
    None
}

pub fn is_pid_running(pid: i32) -> bool {
    // kill(pid, None) tests if process exists and current user has permissions
    kill(Pid::from_raw(pid), None).is_ok()
}

pub fn remove_file_if_exists(path: &Path) {
    if path.is_file() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization_roundtrip() {
        let status = Status {
            active: true,
            state: "recording".to_string(),
            mic_active: true,
            last_transcript: "hey jarvis".to_string(),
            ..Default::default()
        };

        let json = serde_json::to_string(&status).expect("Serialize failed");
        let deserialized: Status = serde_json::from_str(&json).expect("Deserialize failed");

        assert_eq!(status.active, deserialized.active);
        assert_eq!(status.state, deserialized.state);
        assert_eq!(status.mic_active, deserialized.mic_active);
        assert_eq!(status.last_transcript, deserialized.last_transcript);
    }

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_atomic_state_write_and_read() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let status = update_status(|s| {
            s.state = "processing".to_string();
            s.last_transcript = "open spotify".to_string();
        });

        assert_eq!(status.state, "processing");
        assert!(!status.mic_active);

        let read = read_status();
        assert_eq!(read.state, "processing");
        assert_eq!(read.last_transcript, "open spotify");
    }

    #[test]
    fn test_busy_state_management() {
        let _guard = TEST_MUTEX.lock().unwrap();
        set_busy("Analyzing battery metrics...");
        let read = read_status();
        assert!(read.is_busy);
        assert_eq!(read.current_task, "Analyzing battery metrics...");

        clear_busy();
        let read_cleared = read_status();
        assert!(!read_cleared.is_busy);
        assert!(read_cleared.current_task.is_empty());
    }
}
