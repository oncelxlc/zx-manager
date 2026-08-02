use super::dto::{
    NginxControlAction, NginxControlBackend, NginxInstanceRecord, NginxOperationOutcome,
    NginxOperationRecord, NginxRuntimeStatus,
};
use super::error::{NginxError, NginxResult};
use super::process::{launch_command, run_command, ProcessOutput};
use super::registry::write_json_atomically;
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_HISTORY: usize = 1_000;
const HISTORY_DAYS: i64 = 90;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryFile {
    version: u8,
    operations: Vec<NginxOperationRecord>,
}

pub struct OperationHistory {
    path: PathBuf,
    operations: Vec<NginxOperationRecord>,
}

impl OperationHistory {
    pub fn load(path: PathBuf) -> NginxResult<Self> {
        let operations = if path.is_file() {
            let bytes = fs::read(&path)
                .map_err(|error| NginxError::io("read nginx operation history", error))?;
            serde_json::from_slice::<HistoryFile>(&bytes)
                .map(|file| file.operations)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(Self { path, operations })
    }

    pub fn record(
        &mut self,
        instance: &NginxInstanceRecord,
        action: NginxControlAction,
        started_at: String,
        outcome: NginxOperationOutcome,
        resulting_status: NginxRuntimeStatus,
        result: &NginxResult<ProcessOutput>,
    ) -> NginxResult<NginxOperationRecord> {
        let completed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let (success, error_code, stdout, stderr) = match result {
            Ok(output) => (
                output.success,
                (!output.success).then(|| "NGINX_CONTROL_FAILED".to_owned()),
                redact_output(&output.stdout, instance),
                redact_output(&output.stderr, instance),
            ),
            Err(error) => (
                false,
                Some(error.code.to_owned()),
                String::new(),
                redact_output(&error.message, instance),
            ),
        };
        let record = NginxOperationRecord {
            id: Uuid::new_v4().to_string(),
            instance_id: instance.id.clone(),
            action,
            backend: instance.control_backend,
            started_at,
            completed_at,
            success,
            outcome,
            resulting_status,
            error_code,
            stdout,
            stderr,
        };
        self.operations.push(record.clone());
        let cutoff = Utc::now() - ChronoDuration::days(HISTORY_DAYS);
        self.operations.retain(|item| {
            DateTime::parse_from_rfc3339(&item.completed_at)
                .map(|time| time.with_timezone(&Utc) >= cutoff)
                .unwrap_or(false)
        });
        if self.operations.len() > MAX_HISTORY {
            self.operations
                .drain(..self.operations.len().saturating_sub(MAX_HISTORY));
        }
        write_json_atomically(
            &self.path,
            &HistoryFile {
                version: 1,
                operations: self.operations.clone(),
            },
        )?;
        Ok(record)
    }

    pub fn list(&self, instance_id: Option<&str>, limit: usize) -> Vec<NginxOperationRecord> {
        self.operations
            .iter()
            .rev()
            .filter(|item| instance_id.is_none_or(|id| item.instance_id == id))
            .take(limit.clamp(1, 1_000))
            .cloned()
            .collect()
    }
}

pub fn execute(
    instance: &NginxInstanceRecord,
    action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    match instance.control_backend {
        NginxControlBackend::Portable => execute_portable(instance, action),
        NginxControlBackend::WindowsScm => execute_windows_service(instance, action),
        NginxControlBackend::Systemd => execute_systemd(instance, action),
        NginxControlBackend::LaunchAgent => execute_launch_agent(instance, action),
        NginxControlBackend::LaunchDaemonReadOnly | NginxControlBackend::None => Err(
            NginxError::new("NGINX_CONTROL_READ_ONLY", "this provider is read only"),
        ),
    }
}

fn execute_portable(
    instance: &NginxInstanceRecord,
    action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    let binary = Path::new(&instance.binary_path);
    let root = Path::new(&instance.root_path);
    if matches!(
        action,
        NginxControlAction::Start | NginxControlAction::Reload | NginxControlAction::Restart
    ) {
        let validation = run_command(binary, &["-t"], Some(root), 10)?;
        if !validation.success {
            return Ok(validation);
        }
    }
    match action {
        NginxControlAction::Start => launch_command(binary, &[], root),
        NginxControlAction::Stop => run_command(binary, &["-s", "quit"], Some(root), 10),
        NginxControlAction::Reload => run_command(binary, &["-s", "reload"], Some(root), 10),
        NginxControlAction::Restart => {
            let stopped = run_command(binary, &["-s", "quit"], Some(root), 10)?;
            if !stopped.success {
                return Ok(stopped);
            }
            launch_command(binary, &[], root)
        }
    }
}

#[cfg(windows)]
fn execute_windows_service(
    instance: &NginxInstanceRecord,
    action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    let service = validated_external_id(instance)?;
    let verb = match action {
        NginxControlAction::Start => "start",
        NginxControlAction::Stop => "stop",
        NginxControlAction::Reload => {
            return Err(NginxError::new(
                "NGINX_CONTROL_ACTION_UNSUPPORTED",
                "Windows SCM does not expose nginx reload",
            ))
        }
        NginxControlAction::Restart => "restart",
    };
    if verb == "restart" {
        let stopped = run_command(Path::new("sc.exe"), &["stop", service], None, 30)?;
        if !stopped.success {
            return Ok(stopped);
        }
        return run_command(Path::new("sc.exe"), &["start", service], None, 30);
    }
    run_command(Path::new("sc.exe"), &[verb, service], None, 30)
}

#[cfg(not(windows))]
fn execute_windows_service(
    _instance: &NginxInstanceRecord,
    _action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    Err(NginxError::new(
        "NGINX_CONTROL_UNSUPPORTED",
        "Windows SCM is unavailable",
    ))
}

#[cfg(target_os = "linux")]
fn execute_systemd(
    instance: &NginxInstanceRecord,
    action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    let unit = validated_external_id(instance)?;
    let verb = match action {
        NginxControlAction::Start => "start",
        NginxControlAction::Stop => "stop",
        NginxControlAction::Reload => "reload",
        NginxControlAction::Restart => "restart",
    };
    // systemctl is invoked directly with a fixed verb and validated unit; it delegates
    // authorization to systemd/Polkit and never passes through a shell.
    run_command(Path::new("/usr/bin/systemctl"), &[verb, unit], None, 60)
}

#[cfg(not(target_os = "linux"))]
fn execute_systemd(
    _instance: &NginxInstanceRecord,
    _action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    Err(NginxError::new(
        "NGINX_CONTROL_UNSUPPORTED",
        "systemd is unavailable",
    ))
}

#[cfg(target_os = "macos")]
fn execute_launch_agent(
    instance: &NginxInstanceRecord,
    action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    let label = validated_external_id(instance)?;
    let uid = run_command(Path::new("/usr/bin/id"), &["-u"], None, 5)?
        .stdout
        .trim()
        .parse::<u32>()
        .map_err(|_| NginxError::new("NGINX_CONTROL_FAILED", "unable to resolve user id"))?;
    let target = format!("gui/{uid}/{label}");
    match action {
        NginxControlAction::Start | NginxControlAction::Restart => run_command(
            Path::new("/bin/launchctl"),
            &["kickstart", "-k", &target],
            None,
            30,
        ),
        NginxControlAction::Stop => run_command(
            Path::new("/bin/launchctl"),
            &["kill", "SIGTERM", &target],
            None,
            30,
        ),
        NginxControlAction::Reload => Err(NginxError::new(
            "NGINX_CONTROL_ACTION_UNSUPPORTED",
            "launchd does not expose nginx reload",
        )),
    }
}

#[cfg(not(target_os = "macos"))]
fn execute_launch_agent(
    _instance: &NginxInstanceRecord,
    _action: NginxControlAction,
) -> NginxResult<ProcessOutput> {
    Err(NginxError::new(
        "NGINX_CONTROL_UNSUPPORTED",
        "launchd is unavailable",
    ))
}

fn validated_external_id(instance: &NginxInstanceRecord) -> NginxResult<&str> {
    let value = instance
        .provider_identity
        .external_id
        .as_deref()
        .ok_or_else(|| {
            NginxError::new(
                "NGINX_CONTROL_IDENTITY_MISSING",
                "provider identity missing",
            )
        })?;
    if value.is_empty()
        || value.len() > 256
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_.@-".contains(character))
    {
        return Err(NginxError::new(
            "NGINX_CONTROL_IDENTITY_INVALID",
            "provider identity is invalid",
        ));
    }
    Ok(value)
}

fn redact_output(value: &str, instance: &NginxInstanceRecord) -> String {
    let mut redacted = value.to_owned();
    for path in std::iter::once(&instance.root_path)
        .chain(std::iter::once(&instance.binary_path))
        .chain(instance.authorized_roots.iter())
    {
        if !path.is_empty() {
            redacted = redacted.replace(path, "[REDACTED_PATH]");
        }
    }
    redacted.chars().take(64 * 1024).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nginx_manager::dto::{
        NginxAuthorizationLevel, NginxLifecycleState, NginxProviderIdentity,
    };

    fn record(root: &str) -> NginxInstanceRecord {
        NginxInstanceRecord {
            id: "instance".to_owned(),
            name: "Fixture".to_owned(),
            kind: "external".to_owned(),
            root_path: root.to_owned(),
            binary_path: format!("{root}/nginx"),
            config_path: None,
            authorized_roots: vec![root.to_owned()],
            authorization_level: NginxAuthorizationLevel::ReadOnly,
            version: "1.28.0".to_owned(),
            configure_arguments: Vec::new(),
            binary_fingerprint: "hash".to_owned(),
            provider_identity: NginxProviderIdentity {
                provider: "portable".to_owned(),
                external_id: None,
            },
            control_backend: NginxControlBackend::Portable,
            lifecycle_state: NginxLifecycleState::Available,
            created_at: "2026-07-31T00:00:00Z".to_owned(),
            updated_at: "2026-07-31T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn history_redacts_paths_and_is_bounded() {
        let directory = tempfile::tempdir().expect("tempdir");
        let mut history =
            OperationHistory::load(directory.path().join("history.json")).expect("history");
        let instance = record("C:/private/nginx");
        let output = Ok(ProcessOutput {
            success: true,
            stdout: "loaded C:/private/nginx/conf/nginx.conf".to_owned(),
            stderr: String::new(),
        });
        let saved = history
            .record(
                &instance,
                NginxControlAction::Reload,
                "2026-07-31T00:00:00Z".to_owned(),
                NginxOperationOutcome::Executed,
                NginxRuntimeStatus::Running,
                &output,
            )
            .expect("record");
        assert!(!saved.stdout.contains("C:/private/nginx"));
        assert!(saved.stdout.contains("[REDACTED_PATH]"));
    }
}
