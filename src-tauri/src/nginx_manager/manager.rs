use super::dto::{
    AuthorizeNginxRootInput, CheckNginxUpdatesInput, DirectorySelection, DirectorySelectionPurpose,
    NginxAuthorizationLevel, NginxCapabilities, NginxControlBackend, NginxInspection,
    NginxInstance, NginxInstanceRecord, NginxLifecycleState, NginxProviderIdentity,
    NginxReleaseChannel, NginxReleaseStatus, NginxRuntimeStatus, RegisterNginxInstanceInput,
};
use super::error::{NginxError, NginxResult};
use super::process::run_nginx;
use super::registry::NginxRegistry;
use super::release::{is_stale, CachedRelease, ReleaseUpdateService};
use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use semver::Version;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

const TOKEN_TTL: Duration = Duration::from_secs(10 * 60);

struct SelectionToken {
    path: PathBuf,
    purpose: DirectorySelectionPurpose,
    expires_at: Instant,
}

struct InspectionToken {
    inspection: NginxInspection,
    root: PathBuf,
    binary: PathBuf,
    config: Option<PathBuf>,
    expires_at: Instant,
}

pub struct NginxManager {
    registry_path: PathBuf,
    registry: Mutex<NginxRegistry>,
    selections: Mutex<HashMap<String, SelectionToken>>,
    inspections: Mutex<HashMap<String, InspectionToken>>,
    release_updates: ReleaseUpdateService,
}

impl NginxManager {
    pub fn new(data_directory: PathBuf) -> NginxResult<Self> {
        let registry_path = data_directory.join("registry-v1.json");
        let registry = NginxRegistry::load(&registry_path)?;
        let release_updates =
            ReleaseUpdateService::new(data_directory.join("release-cache-v1.json"))?;
        Ok(Self {
            registry_path,
            registry: Mutex::new(registry),
            selections: Mutex::new(HashMap::new()),
            inspections: Mutex::new(HashMap::new()),
            release_updates,
        })
    }

    pub fn create_selection(
        &self,
        path: PathBuf,
        purpose: DirectorySelectionPurpose,
    ) -> NginxResult<DirectorySelection> {
        reject_reparse_points(&path)?;
        let canonical = path
            .canonicalize()
            .map_err(|error| NginxError::io("canonicalize selected directory", error))?;
        if !canonical.is_dir() {
            return Err(NginxError::new(
                "NGINX_SELECTION_NOT_DIRECTORY",
                "the selected path is not a directory",
            ));
        }
        let selection_id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + ChronoDuration::from_std(TOKEN_TTL).unwrap_or_default();
        self.selections.lock().unwrap().insert(
            selection_id.clone(),
            SelectionToken {
                path: canonical.clone(),
                purpose,
                expires_at: Instant::now() + TOKEN_TTL,
            },
        );
        Ok(DirectorySelection {
            selection_id,
            display_path: canonical.to_string_lossy().into_owned(),
            expires_at: expires_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        })
    }

    pub fn inspect(&self, selection_id: &str) -> NginxResult<NginxInspection> {
        let selection =
            self.consume_selection(selection_id, DirectorySelectionPurpose::InspectInstance)?;
        let binary = locate_binary(&selection.path)?;
        reject_reparse_points(&binary)?;
        let version_output = run_nginx(&binary, &["-v"])?;
        let build_output = run_nginx(&binary, &["-V"])?;
        if !version_output.success || !build_output.success {
            return Err(NginxError::new(
                "NGINX_INSPECTION_COMMAND_FAILED",
                sanitize_process_message(&build_output.stderr),
            ));
        }
        let combined = format!(
            "{}\n{}\n{}\n{}",
            version_output.stdout, version_output.stderr, build_output.stdout, build_output.stderr
        );
        let version = parse_version(&combined).ok_or_else(|| {
            NginxError::new(
                "NGINX_VERSION_UNRECOGNIZED",
                "unable to parse nginx version",
            )
        })?;
        let configure_arguments = parse_configure_arguments(&combined);
        let configured_path = parse_config_path(&configure_arguments, &selection.path);
        let (config, warnings) = authorize_config_path(configured_path, &selection.path);
        let inspection_id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + ChronoDuration::from_std(TOKEN_TTL).unwrap_or_default();
        let inspection = NginxInspection {
            inspection_id: inspection_id.clone(),
            display_root: selection.path.to_string_lossy().into_owned(),
            display_binary: binary.to_string_lossy().into_owned(),
            version,
            configure_arguments,
            config_path: config
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            binary_fingerprint: fingerprint(&binary)?,
            warnings,
            expires_at: expires_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        };
        self.inspections.lock().unwrap().insert(
            inspection_id,
            InspectionToken {
                inspection: inspection.clone(),
                root: selection.path,
                binary,
                config,
                expires_at: Instant::now() + TOKEN_TTL,
            },
        );
        Ok(inspection)
    }

    pub fn register(&self, input: RegisterNginxInstanceInput) -> NginxResult<NginxInstance> {
        let token = self
            .inspections
            .lock()
            .unwrap()
            .remove(&input.inspection_id)
            .filter(|token| token.expires_at > Instant::now())
            .ok_or_else(expired_token_error)?;
        let name = input.name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(NginxError::new(
                "NGINX_INSTANCE_NAME_INVALID",
                "instance name must contain between 1 and 80 characters",
            ));
        }
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let record = NginxInstanceRecord {
            id: Uuid::new_v4().to_string(),
            name: name.to_owned(),
            kind: "external".to_owned(),
            root_path: token.root.to_string_lossy().into_owned(),
            binary_path: token.binary.to_string_lossy().into_owned(),
            config_path: token.config.map(|path| path.to_string_lossy().into_owned()),
            authorized_roots: vec![token.root.to_string_lossy().into_owned()],
            authorization_level: input.authorization_level,
            version: token.inspection.version,
            configure_arguments: token.inspection.configure_arguments,
            binary_fingerprint: token.inspection.binary_fingerprint,
            provider_identity: NginxProviderIdentity {
                provider: "portable".to_owned(),
                external_id: None,
            },
            control_backend: NginxControlBackend::Portable,
            lifecycle_state: NginxLifecycleState::Available,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut registry = self.registry.lock().unwrap();
        if registry.instances.iter().any(|existing| {
            existing
                .binary_path
                .eq_ignore_ascii_case(&record.binary_path)
        }) {
            return Err(NginxError::new(
                "NGINX_INSTANCE_ALREADY_REGISTERED",
                "this nginx binary is already registered",
            ));
        }
        registry.instances.push(record.clone());
        registry.save(&self.registry_path)?;
        Ok(to_instance(record))
    }

    pub fn list(&self) -> Vec<NginxInstance> {
        self.registry
            .lock()
            .unwrap()
            .instances
            .iter()
            .cloned()
            .map(refresh_instance)
            .collect()
    }

    pub fn refresh(&self, instance_id: &str) -> NginxResult<NginxInstance> {
        let record = self
            .registry
            .lock()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .cloned()
            .ok_or_else(instance_not_found)?;
        Ok(refresh_instance(record))
    }

    pub fn unregister(&self, instance_id: &str) -> NginxResult<()> {
        let mut registry = self.registry.lock().unwrap();
        let previous_len = registry.instances.len();
        registry
            .instances
            .retain(|instance| instance.id != instance_id);
        if registry.instances.len() == previous_len {
            return Err(instance_not_found());
        }
        registry.save(&self.registry_path)
    }

    pub fn authorize_root(&self, input: AuthorizeNginxRootInput) -> NginxResult<NginxInstance> {
        let selection = self.consume_selection(
            &input.selection_id,
            DirectorySelectionPurpose::AuthorizeAdditionalRoot,
        )?;
        let mut registry = self.registry.lock().unwrap();
        let record = registry
            .instances
            .iter_mut()
            .find(|instance| instance.id == input.instance_id)
            .ok_or_else(instance_not_found)?;
        let display_path = selection.path.to_string_lossy().into_owned();
        if !record
            .authorized_roots
            .iter()
            .any(|root| root.eq_ignore_ascii_case(&display_path))
        {
            record.authorized_roots.push(display_path);
            record.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        }
        let result = record.clone();
        registry.save(&self.registry_path)?;
        Ok(refresh_instance(result))
    }

    pub fn release_status(&self, channel: NginxReleaseChannel) -> NginxReleaseStatus {
        self.build_release_status(channel, self.release_updates.cached(channel), "cache")
    }

    pub async fn check_updates(
        &self,
        input: CheckNginxUpdatesInput,
    ) -> NginxResult<NginxReleaseStatus> {
        let (cached, fetched) = self
            .release_updates
            .check(input.channel, input.force, input.max_age_hours)
            .await?;
        Ok(self.build_release_status(
            input.channel,
            Some(cached),
            if fetched { "network" } else { "cache" },
        ))
    }

    fn build_release_status(
        &self,
        channel: NginxReleaseChannel,
        cached: Option<CachedRelease>,
        source: &str,
    ) -> NginxReleaseStatus {
        let latest = cached
            .as_ref()
            .and_then(|value| Version::parse(&value.release.version).ok());
        let outdated_instance_ids = latest
            .map(|latest| {
                self.registry
                    .lock()
                    .unwrap()
                    .instances
                    .iter()
                    .filter(|instance| {
                        Version::parse(&instance.version)
                            .map(|version| version < latest)
                            .unwrap_or(false)
                    })
                    .map(|instance| instance.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        NginxReleaseStatus {
            channel,
            latest_release: cached.as_ref().map(|value| value.release.clone()),
            checked_at: cached.as_ref().map(|value| value.checked_at.clone()),
            stale: cached
                .as_ref()
                .map(|value| is_stale(&value.checked_at, 24))
                .unwrap_or(true),
            source: if cached.is_some() {
                source.to_owned()
            } else {
                "none".to_owned()
            },
            update_available_count: outdated_instance_ids.len(),
            outdated_instance_ids,
        }
    }

    fn consume_selection(
        &self,
        selection_id: &str,
        expected_purpose: DirectorySelectionPurpose,
    ) -> NginxResult<SelectionToken> {
        let token = self
            .selections
            .lock()
            .unwrap()
            .remove(selection_id)
            .filter(|token| token.expires_at > Instant::now())
            .ok_or_else(expired_token_error)?;
        if std::mem::discriminant(&token.purpose) != std::mem::discriminant(&expected_purpose) {
            return Err(NginxError::new(
                "NGINX_SELECTION_PURPOSE_MISMATCH",
                "the directory selection cannot be used for this operation",
            ));
        }
        Ok(token)
    }
}

fn locate_binary(root: &Path) -> NginxResult<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![root.join("nginx.exe"), root.join("sbin/nginx.exe")]
    } else {
        vec![root.join("nginx"), root.join("sbin/nginx")]
    };
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| NginxError::new("NGINX_BINARY_NOT_FOUND", "no nginx binary was found"))
}

fn parse_version(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| {
            line.split_once("nginx version: nginx/")
                .map(|(_, value)| value.trim())
        })
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_configure_arguments(output: &str) -> Vec<String> {
    output
        .lines()
        .find_map(|line| {
            line.split_once("configure arguments:")
                .map(|(_, value)| value)
        })
        .map(|value| value.split_whitespace().map(ToOwned::to_owned).collect())
        .unwrap_or_default()
}

fn parse_config_path(arguments: &[String], root: &Path) -> Option<PathBuf> {
    let configured = arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--conf-path="))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("conf/nginx.conf"));
    Some(if configured.is_absolute() {
        configured
    } else {
        root.join(configured)
    })
}

fn authorize_config_path(path: Option<PathBuf>, root: &Path) -> (Option<PathBuf>, Vec<String>) {
    let Some(path) = path else {
        return (None, Vec::new());
    };
    let Ok(canonical) = path.canonicalize() else {
        return (None, vec!["NGINX_CONFIG_NOT_FOUND".to_owned()]);
    };
    if !canonical.starts_with(root) || reject_reparse_points(&canonical).is_err() {
        return (None, vec!["NGINX_CONFIG_ROOT_NOT_AUTHORIZED".to_owned()]);
    }
    (Some(canonical), Vec::new())
}

fn fingerprint(path: &Path) -> NginxResult<String> {
    let bytes = fs::read(path).map_err(|error| NginxError::io("read nginx binary", error))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn refresh_instance(mut record: NginxInstanceRecord) -> NginxInstance {
    let binary = Path::new(&record.binary_path);
    record.lifecycle_state = if !binary.is_file() {
        NginxLifecycleState::Missing
    } else if fingerprint(binary)
        .map(|current| current != record.binary_fingerprint)
        .unwrap_or(true)
    {
        NginxLifecycleState::Changed
    } else {
        NginxLifecycleState::Available
    };
    to_instance(record)
}

fn to_instance(record: NginxInstanceRecord) -> NginxInstance {
    let available = record.lifecycle_state == NginxLifecycleState::Available;
    NginxInstance {
        capabilities: NginxCapabilities {
            can_read: available,
            can_edit: available && record.authorization_level == NginxAuthorizationLevel::Full,
            can_control: false,
            can_unregister: true,
        },
        runtime_status: if available {
            detect_runtime_status(&record)
        } else if record.lifecycle_state == NginxLifecycleState::Missing {
            NginxRuntimeStatus::Stopped
        } else {
            NginxRuntimeStatus::Unknown
        },
        record,
    }
}

fn detect_runtime_status(record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    use sysinfo::{Pid, ProcessesToUpdate, System};

    let pid_path = record
        .configure_arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--pid-path="))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("logs/nginx.pid"));
    let pid_path = if pid_path.is_absolute() {
        pid_path
    } else {
        Path::new(&record.root_path).join(pid_path)
    };
    let Ok(canonical_pid_path) = pid_path.canonicalize() else {
        return NginxRuntimeStatus::Stopped;
    };
    if !record
        .authorized_roots
        .iter()
        .any(|root| canonical_pid_path.starts_with(Path::new(root)))
    {
        return NginxRuntimeStatus::Unknown;
    }
    let Ok(metadata) = fs::metadata(&canonical_pid_path) else {
        return NginxRuntimeStatus::Unknown;
    };
    if metadata.len() > 64 || reject_reparse_points(&canonical_pid_path).is_err() {
        return NginxRuntimeStatus::Unknown;
    }
    let Ok(contents) = fs::read_to_string(canonical_pid_path) else {
        return NginxRuntimeStatus::Unknown;
    };
    let Ok(pid) = contents.trim().parse::<usize>() else {
        return NginxRuntimeStatus::Unknown;
    };
    let pid = Pid::from(pid);
    let mut system = System::new();
    if system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true) == 0 {
        return NginxRuntimeStatus::Stopped;
    }
    let Some(executable) = system.process(pid).and_then(|process| process.exe()) else {
        return NginxRuntimeStatus::Unknown;
    };
    executable_identity_status(executable, Path::new(&record.binary_path))
}

fn executable_identity_status(actual: &Path, expected: &Path) -> NginxRuntimeStatus {
    match (actual.canonicalize(), expected.canonicalize()) {
        (Ok(actual), Ok(expected)) if actual == expected => NginxRuntimeStatus::Running,
        (Ok(_), Ok(_)) => NginxRuntimeStatus::Stopped,
        _ => NginxRuntimeStatus::Unknown,
    }
}

fn sanitize_process_message(message: &str) -> String {
    let value = message.lines().next().unwrap_or_default().trim();
    if value.is_empty() {
        "nginx returned an unsuccessful exit code".to_owned()
    } else {
        value.chars().take(512).collect()
    }
}

fn expired_token_error() -> NginxError {
    NginxError::new(
        "NGINX_TOKEN_INVALID_OR_EXPIRED",
        "the opaque selection or inspection token is invalid or expired",
    )
}

fn instance_not_found() -> NginxError {
    NginxError::new("NGINX_INSTANCE_NOT_FOUND", "nginx instance was not found")
}

fn reject_reparse_points(path: &Path) -> NginxResult<()> {
    for ancestor in path.ancestors() {
        if !ancestor.exists() {
            continue;
        }
        let metadata = fs::symlink_metadata(ancestor)
            .map_err(|error| NginxError::io("inspect selected path", error))?;
        if metadata.file_type().is_symlink() || has_windows_reparse_attribute(&metadata) {
            return Err(NginxError::new(
                "NGINX_REPARSE_POINT_REJECTED",
                "symbolic links and reparse points are not accepted as authorization roots",
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn has_windows_reparse_attribute(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn has_windows_reparse_attribute(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(root: &Path, binary: &Path, fingerprint: String) -> NginxInstanceRecord {
        NginxInstanceRecord {
            id: "instance".to_owned(),
            name: "Nginx".to_owned(),
            kind: "external".to_owned(),
            root_path: root.to_string_lossy().into_owned(),
            binary_path: binary.to_string_lossy().into_owned(),
            config_path: None,
            authorized_roots: vec![root.to_string_lossy().into_owned()],
            authorization_level: NginxAuthorizationLevel::ReadOnly,
            version: "1.28.0".to_owned(),
            configure_arguments: Vec::new(),
            binary_fingerprint: fingerprint,
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
    fn parses_version_and_configure_arguments() {
        let output = "nginx version: nginx/1.28.0\nconfigure arguments: --prefix=/srv/nginx --with-http_ssl_module";
        assert_eq!(parse_version(output).as_deref(), Some("1.28.0"));
        assert_eq!(
            parse_configure_arguments(output),
            vec!["--prefix=/srv/nginx", "--with-http_ssl_module"]
        );
    }

    #[test]
    fn selection_token_is_single_use() {
        let directory = tempfile::tempdir().expect("tempdir");
        let manager = NginxManager::new(directory.path().join("data")).expect("manager");
        let selection = manager
            .create_selection(
                directory.path().to_path_buf(),
                DirectorySelectionPurpose::InspectInstance,
            )
            .expect("selection");
        let _ = manager.consume_selection(
            &selection.selection_id,
            DirectorySelectionPurpose::InspectInstance,
        );
        assert!(manager
            .consume_selection(
                &selection.selection_id,
                DirectorySelectionPurpose::InspectInstance,
            )
            .is_err());
    }

    #[test]
    fn changed_binary_loses_registered_capabilities() {
        let directory = tempfile::tempdir().expect("tempdir");
        let binary = directory
            .path()
            .join(if cfg!(windows) { "nginx.exe" } else { "nginx" });
        fs::write(&binary, b"registered").expect("binary fixture");
        let registered_fingerprint = fingerprint(&binary).expect("fingerprint");
        fs::write(&binary, b"replaced").expect("replace fixture");

        let instance = refresh_instance(record(directory.path(), &binary, registered_fingerprint));

        assert_eq!(
            instance.record.lifecycle_state,
            NginxLifecycleState::Changed
        );
        assert!(!instance.capabilities.can_read);
        assert!(!instance.capabilities.can_edit);
    }

    #[test]
    fn runtime_status_requires_executable_identity_match() {
        let directory = tempfile::tempdir().expect("tempdir");
        let registered_binary = directory.path().join("nginx-binary");
        let different_binary = directory.path().join("different-binary");
        fs::write(&registered_binary, b"registered").expect("registered fixture");
        fs::write(&different_binary, b"different").expect("different fixture");

        assert_eq!(
            executable_identity_status(&registered_binary, &registered_binary),
            NginxRuntimeStatus::Running
        );
        assert_eq!(
            executable_identity_status(&registered_binary, &different_binary),
            NginxRuntimeStatus::Stopped
        );
    }
}
