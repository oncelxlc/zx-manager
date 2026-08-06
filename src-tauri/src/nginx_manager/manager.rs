use super::configuration::load_configuration;
use super::control::{execute as execute_control, OperationHistory};
use super::dto::{
    AuthorizeNginxRootInput, CheckNginxUpdatesInput, ControlNginxInstanceInput, DirectorySelection,
    DirectorySelectionPurpose, GetNginxOperationHistoryInput, InspectNginxSystemServiceInput,
    NginxAuthorizationLevel, NginxCapabilities, NginxConfiguration, NginxControlBackend,
    NginxInspection, NginxInstance, NginxInstanceRecord, NginxLifecycleState,
    NginxOperationOutcome, NginxOperationPhase, NginxOperationRecord, NginxProcessRole,
    NginxProviderIdentity, NginxRegistryState, NginxRegistryStatus, NginxReleaseChannel,
    NginxReleaseStatus, NginxRuntimeDetails, NginxRuntimeMetricAvailability, NginxRuntimeProcess,
    NginxRuntimeStatus, NginxStatusEvent, NginxStatusSubscription, NginxSystemServiceCandidate,
    NginxSystemServiceInspection, NginxUpgradeProgress, NginxUpgradeResult,
    RegisterNginxInstanceInput, RegisterNginxSystemServiceInput,
    ResolveNginxRegistryMigrationInput, UpgradeNginxInstanceInput,
};
use super::error::{NginxError, NginxResult};
use super::process::{run_command, run_nginx, ProcessOutput};
use super::registry::NginxRegistry;
use super::release::{is_stale, CachedRelease, ReleaseUpdateService};
use super::upgrade::{
    backup_record, cleanup_backups, create_backup, extract_release, replace_binary, restore_binary,
    verify_signature,
};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use semver::Version;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::ipc::Channel;
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

#[derive(Clone)]
struct ServiceDiscoveryToken {
    display_name: String,
    external_id: String,
    binary: PathBuf,
    backend: NginxControlBackend,
    domain: String,
    read_only: bool,
    expires_at: Instant,
}

struct ServiceInspectionToken {
    instance_id: String,
    discovery: ServiceDiscoveryToken,
    expires_at: Instant,
}

#[derive(Debug)]
struct OperationGuard {
    flag: Arc<AtomicBool>,
}

impl OperationGuard {
    fn try_acquire(flag: Arc<AtomicBool>) -> NginxResult<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                NginxError::new(
                    "NGINX_OPERATION_IN_PROGRESS",
                    "another nginx operation is already in progress",
                )
            })?;
        Ok(Self { flag })
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
    }
}

pub struct NginxManager {
    data_directory: PathBuf,
    registry_path: PathBuf,
    registry: Arc<Mutex<NginxRegistry>>,
    selections: Mutex<HashMap<String, SelectionToken>>,
    inspections: Mutex<HashMap<String, InspectionToken>>,
    service_discoveries: Mutex<HashMap<String, ServiceDiscoveryToken>>,
    service_inspections: Mutex<HashMap<String, ServiceInspectionToken>>,
    operation_locks: Mutex<HashMap<String, Arc<AtomicBool>>>,
    operation_history: Mutex<OperationHistory>,
    release_updates: ReleaseUpdateService,
    status_subscribers: Arc<Mutex<HashMap<u64, Channel<NginxStatusEvent>>>>,
    next_subscription_id: AtomicU64,
    status_generation: Arc<AtomicU64>,
    status_sequence: Arc<AtomicU64>,
    runtime_system: Arc<Mutex<System>>,
    operation_phase: Arc<Mutex<Option<NginxOperationPhase>>>,
    monitor_shutdown: Arc<AtomicBool>,
    monitor_handle: Mutex<Option<JoinHandle<()>>>,
}

impl NginxManager {
    pub fn new(data_directory: PathBuf) -> NginxResult<Self> {
        let registry_path = data_directory.join("registry-v2.json");
        let registry =
            NginxRegistry::load(&registry_path, &data_directory.join("registry-v1.json"))?;
        let release_updates =
            ReleaseUpdateService::new(data_directory.join("release-cache-v1.json"))?;
        let operation_history =
            OperationHistory::load(data_directory.join("operation-history-v1.json"))?;
        Ok(Self {
            data_directory,
            registry_path,
            registry: Arc::new(Mutex::new(registry)),
            selections: Mutex::new(HashMap::new()),
            inspections: Mutex::new(HashMap::new()),
            service_discoveries: Mutex::new(HashMap::new()),
            service_inspections: Mutex::new(HashMap::new()),
            operation_locks: Mutex::new(HashMap::new()),
            operation_history: Mutex::new(operation_history),
            release_updates,
            status_subscribers: Arc::new(Mutex::new(HashMap::new())),
            next_subscription_id: AtomicU64::new(1),
            status_generation: Arc::new(AtomicU64::new(1)),
            status_sequence: Arc::new(AtomicU64::new(0)),
            runtime_system: Arc::new(Mutex::new(System::new())),
            operation_phase: Arc::new(Mutex::new(None)),
            monitor_shutdown: Arc::new(AtomicBool::new(false)),
            monitor_handle: Mutex::new(None),
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
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let record = NginxInstanceRecord {
            id: Uuid::new_v4().to_string(),
            name: "Nginx".to_owned(),
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
        registry.insert(&self.registry_path, record.clone())?;
        self.bump_status_generation();
        Ok(to_instance(record))
    }

    pub fn registry_state(&self) -> NginxRegistryState {
        let registry = self.registry.lock().unwrap();
        if registry.migration_required() {
            return NginxRegistryState {
                status: NginxRegistryStatus::MigrationRequired,
                instance: None,
                migration_candidates: registry
                    .migration_candidates()
                    .iter()
                    .cloned()
                    .map(refresh_instance)
                    .collect(),
            };
        }
        let instance = registry.instance.clone().map(refresh_instance);
        NginxRegistryState {
            status: if instance.is_some() {
                NginxRegistryStatus::Ready
            } else {
                NginxRegistryStatus::Empty
            },
            instance,
            migration_candidates: Vec::new(),
        }
    }

    pub fn resolve_registry_migration(
        &self,
        input: ResolveNginxRegistryMigrationInput,
    ) -> NginxResult<NginxRegistryState> {
        self.registry
            .lock()
            .unwrap()
            .resolve_migration(&self.registry_path, &input.keep_instance_id)?;
        self.bump_status_generation();
        Ok(self.registry_state())
    }

    pub fn refresh(&self, instance_id: &str) -> NginxResult<NginxInstance> {
        let record = self.get_record(instance_id)?;
        Ok(refresh_instance(record))
    }

    pub fn unregister(&self, instance_id: &str) -> NginxResult<()> {
        let mut registry = self.registry.lock().unwrap();
        if registry.migration_required() {
            return Err(migration_required());
        }
        if registry.instance.as_ref().map(|value| value.id.as_str()) != Some(instance_id) {
            return Err(instance_not_found());
        }
        registry.instance = None;
        registry.save(&self.registry_path)?;
        self.bump_status_generation();
        Ok(())
    }

    pub fn authorize_root(&self, input: AuthorizeNginxRootInput) -> NginxResult<NginxInstance> {
        let selection = self.consume_selection(
            &input.selection_id,
            DirectorySelectionPurpose::AuthorizeAdditionalRoot,
        )?;
        let mut registry = self.registry.lock().unwrap();
        let record = registry
            .instance
            .as_mut()
            .filter(|instance| instance.id == input.instance_id)
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

    pub fn configuration(&self, instance_id: &str) -> NginxResult<NginxConfiguration> {
        let record = self.get_record(instance_id)?;
        if refresh_instance(record.clone()).capabilities.can_read {
            load_configuration(&record)
        } else {
            Err(NginxError::new(
                "NGINX_INSTANCE_NOT_READABLE",
                "the registered instance is not currently trusted for reads",
            ))
        }
    }

    pub fn control(&self, input: ControlNginxInstanceInput) -> NginxResult<NginxOperationRecord> {
        let record = self.get_record(&input.instance_id)?;
        if !refresh_instance(record.clone()).capabilities.can_control {
            return Err(NginxError::new(
                "NGINX_CONTROL_NOT_AUTHORIZED",
                "the instance is not authorized for control",
            ));
        }
        let transaction_lock = self
            .operation_locks
            .lock()
            .unwrap()
            .entry(record.id.clone())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone();
        let _transaction = OperationGuard::try_acquire(transaction_lock)?;
        let phase = match input.action {
            super::dto::NginxControlAction::Start => NginxOperationPhase::Starting,
            super::dto::NginxControlAction::Stop => NginxOperationPhase::Stopping,
            super::dto::NginxControlAction::Reload => NginxOperationPhase::Reloading,
            super::dto::NginxControlAction::Restart => NginxOperationPhase::Restarting,
        };
        let started_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let before = detect_runtime_status(&record);
        let outcome = control_outcome(input.action, before)?;
        if outcome == NginxOperationOutcome::Executed {
            *self.operation_phase.lock().unwrap() = Some(phase);
            self.emit_current_status(false);
        }
        let mut result = if outcome == NginxOperationOutcome::Executed {
            execute_control(&record, input.action)
        } else {
            Ok(ProcessOutput {
                success: true,
                stdout: "nginx already has the requested runtime state".to_owned(),
                stderr: String::new(),
            })
        };
        let target = match input.action {
            super::dto::NginxControlAction::Stop => NginxRuntimeStatus::Stopped,
            _ => NginxRuntimeStatus::Running,
        };
        let resulting_status = if outcome == NginxOperationOutcome::Noop {
            before
        } else if result.as_ref().is_ok_and(|output| output.success) {
            wait_for_runtime_status(&record, target, Duration::from_secs(30))
        } else {
            detect_runtime_status(&record)
        };
        if result.as_ref().is_ok_and(|output| output.success) && resulting_status != target {
            result = Err(NginxError::new(
                "NGINX_RUNTIME_TRANSITION_TIMEOUT",
                "nginx did not reach the requested runtime state",
            ));
        }
        let operation = self.operation_history.lock().unwrap().record(
            &record,
            input.action,
            started_at,
            outcome,
            resulting_status,
            &result,
        );
        if outcome == NginxOperationOutcome::Executed {
            *self.operation_phase.lock().unwrap() = None;
            self.emit_current_status(true);
        }
        let operation = operation?;
        match result {
            Ok(output) if output.success => Ok(operation),
            Ok(_) => Err(NginxError::new(
                "NGINX_CONTROL_FAILED",
                "nginx rejected the requested control operation",
            )),
            Err(error) => Err(error),
        }
    }

    pub fn operation_history(
        &self,
        input: GetNginxOperationHistoryInput,
    ) -> Vec<NginxOperationRecord> {
        self.operation_history
            .lock()
            .unwrap()
            .list(input.instance_id.as_deref(), input.limit)
    }

    pub fn runtime_details(&self, instance_id: &str) -> NginxResult<NginxRuntimeDetails> {
        let record = self.get_record(instance_id)?;
        if record.lifecycle_state != NginxLifecycleState::Available {
            return Err(NginxError::new(
                "NGINX_INSTANCE_NOT_READABLE",
                "the registered nginx executable is not available",
            ));
        }
        let status = detect_runtime_status(&record);
        Ok(collect_runtime_details(
            &record,
            status,
            &mut self.runtime_system.lock().unwrap(),
        ))
    }

    pub async fn upgrade(
        &self,
        input: UpgradeNginxInstanceInput,
        progress_channel: Channel<NginxUpgradeProgress>,
    ) -> NginxResult<NginxUpgradeResult> {
        if !cfg!(windows) {
            return Err(NginxError::new(
                "NGINX_UPGRADE_PLATFORM_UNSUPPORTED",
                "managed nginx upgrade is only available on Windows",
            ));
        }
        if !(1..=50).contains(&input.backup_retention_count) {
            return Err(NginxError::new(
                "NGINX_UPGRADE_RETENTION_INVALID",
                "backup retention must be between 1 and 50",
            ));
        }
        let record = self.get_record(&input.instance_id)?;
        let refreshed = refresh_instance(record.clone());
        if record.control_backend != NginxControlBackend::Portable
            || record.authorization_level != NginxAuthorizationLevel::Full
            || refreshed.record.lifecycle_state != NginxLifecycleState::Available
        {
            return Err(NginxError::new(
                "NGINX_UPGRADE_NOT_SUPPORTED",
                "this nginx instance is not eligible for managed upgrade",
            ));
        }
        let current = Version::parse(&record.version).map_err(|_| {
            NginxError::new(
                "NGINX_UPGRADE_VERSION_INVALID",
                "current nginx version is not compatible with managed upgrade",
            )
        })?;
        let target = Version::parse(&input.target_version).map_err(|_| {
            NginxError::new(
                "NGINX_UPGRADE_VERSION_INVALID",
                "target nginx version is invalid",
            )
        })?;
        if target <= current {
            return Err(NginxError::new(
                "NGINX_UPGRADE_VERSION_NOT_NEWER",
                "target nginx version must be newer than the installed version",
            ));
        }
        let release = self
            .release_updates
            .cached(input.channel)
            .filter(|cached| cached.release.version == input.target_version)
            .map(|cached| cached.release)
            .ok_or_else(|| {
                NginxError::new(
                    "NGINX_UPGRADE_RELEASE_NOT_CACHED",
                    "check for official nginx updates before upgrading",
                )
            })?;
        if !release.download_url.ends_with(".zip") || !release.signature_url.ends_with(".zip.asc") {
            return Err(NginxError::new(
                "NGINX_UPGRADE_ARTIFACT_UNSUPPORTED",
                "cached release is not an official Windows ZIP",
            ));
        }
        let transaction_lock = self
            .operation_locks
            .lock()
            .unwrap()
            .entry(record.id.clone())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone();
        let _transaction = OperationGuard::try_acquire(transaction_lock)?;

        let from_version = record.version.clone();
        let backup_parent = self.data_directory.join("backups");
        let staging_parent = self.data_directory.join("staging");
        let original_status = detect_runtime_status(&record);
        if matches!(
            original_status,
            NginxRuntimeStatus::Conflict | NginxRuntimeStatus::Unknown
        ) {
            return Err(NginxError::new(
                "NGINX_RUNTIME_STATE_INVALID",
                "nginx runtime state must be verifiable before upgrade",
            ));
        }
        let was_running = original_status == NginxRuntimeStatus::Running;
        let mut backup_id: Option<String> = None;
        let mut staging_directory: Option<PathBuf> = None;
        let mut replaced = false;
        let mut stopped_for_upgrade = false;

        let upgrade_result: NginxResult<()> = async {
            self.set_upgrade_phase(
                &progress_channel,
                NginxOperationPhase::Downloading,
                5,
                "upgrade.progress.downloading",
            );
            let (archive, signature) = self.release_updates.download_release(&release).await?;
            self.set_upgrade_phase(
                &progress_channel,
                NginxOperationPhase::Verifying,
                25,
                "upgrade.progress.verifying",
            );
            verify_signature(&archive, &signature)?;
            let prepared = extract_release(&archive, &staging_parent, &input.target_version)?;
            staging_directory = Some(prepared.staging_directory.clone());
            let version_output = run_nginx(&prepared.candidate_binary, &["-V"])?;
            let candidate_version = parse_version(&format!(
                "{}\n{}",
                version_output.stdout, version_output.stderr
            ));
            if !version_output.success
                || candidate_version.as_deref() != Some(&input.target_version)
            {
                return Err(NginxError::new(
                    "NGINX_UPGRADE_BINARY_VERSION_MISMATCH",
                    "candidate nginx.exe did not report the target version",
                ));
            }
            validate_candidate_configuration(&prepared.candidate_binary, &record)?;
            self.set_upgrade_phase(
                &progress_channel,
                NginxOperationPhase::BackingUp,
                45,
                "upgrade.progress.backingUp",
            );
            backup_id = Some(create_backup(&backup_parent, &record, original_status)?);
            if was_running {
                self.set_upgrade_phase(
                    &progress_channel,
                    NginxOperationPhase::Stopping,
                    55,
                    "upgrade.progress.stopping",
                );
                let output = execute_control(&record, super::dto::NginxControlAction::Stop)?;
                if !output.success
                    || wait_for_runtime_status(
                        &record,
                        NginxRuntimeStatus::Stopped,
                        Duration::from_secs(30),
                    ) != NginxRuntimeStatus::Stopped
                {
                    return Err(NginxError::new(
                        "NGINX_UPGRADE_STOP_FAILED",
                        "nginx could not be stopped before replacement",
                    ));
                }
                stopped_for_upgrade = true;
            }
            self.set_upgrade_phase(
                &progress_channel,
                NginxOperationPhase::Replacing,
                70,
                "upgrade.progress.replacing",
            );
            replace_binary(&prepared.candidate_binary, Path::new(&record.binary_path))?;
            replaced = true;
            validate_installed_upgrade(&record, &input.target_version)?;
            let mut updated = record.clone();
            updated.version = input.target_version.clone();
            updated.binary_fingerprint = fingerprint(Path::new(&updated.binary_path))?;
            updated.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
            {
                let mut registry = self.registry.lock().unwrap();
                registry.instance = Some(updated.clone());
                registry.save(&self.registry_path)?;
            }
            if was_running {
                self.set_upgrade_phase(
                    &progress_channel,
                    NginxOperationPhase::RestoringRuntime,
                    90,
                    "upgrade.progress.starting",
                );
                let output = execute_control(&updated, super::dto::NginxControlAction::Start)?;
                if !output.success
                    || wait_for_runtime_status(
                        &updated,
                        NginxRuntimeStatus::Running,
                        Duration::from_secs(30),
                    ) != NginxRuntimeStatus::Running
                {
                    return Err(NginxError::new(
                        "NGINX_UPGRADE_RESTART_FAILED",
                        "upgraded nginx could not be started",
                    ));
                }
            }
            Ok(())
        }
        .await;

        let mut final_result = match upgrade_result {
            Ok(()) => {
                let _ = cleanup_backups(&backup_parent, input.backup_retention_count, None);
                NginxUpgradeResult {
                    from_version,
                    target_version: input.target_version,
                    backup_id: backup_id.clone(),
                    success: true,
                    rolled_back: false,
                    rollback_succeeded: None,
                    error_code: None,
                    resulting_status: self
                        .get_record(&input.instance_id)
                        .map(|current| detect_runtime_status(&current))
                        .unwrap_or(NginxRuntimeStatus::Unknown),
                }
            }
            Err(error) if replaced => {
                self.set_upgrade_phase(
                    &progress_channel,
                    NginxOperationPhase::RollingBack,
                    95,
                    "upgrade.progress.rollingBack",
                );
                // `replaced` is set only after the mandatory snapshot succeeds.
                let id = backup_id.as_deref().unwrap_or_default();
                let rollback = self.rollback_upgrade(&backup_parent, id, was_running);
                let rollback_succeeded = rollback.is_ok();
                let error_code = if rollback_succeeded {
                    error.code
                } else {
                    "NGINX_UPGRADE_ROLLBACK_FAILED"
                };
                let _ = cleanup_backups(&backup_parent, input.backup_retention_count, Some(id));
                NginxUpgradeResult {
                    from_version,
                    target_version: input.target_version,
                    backup_id: backup_id.clone(),
                    success: false,
                    rolled_back: true,
                    rollback_succeeded: Some(rollback_succeeded),
                    error_code: Some(error_code.to_owned()),
                    resulting_status: self
                        .get_record(&input.instance_id)
                        .map(|current| detect_runtime_status(&current))
                        .unwrap_or(NginxRuntimeStatus::Unknown),
                }
            }
            Err(error) if backup_id.is_some() => {
                let runtime_restored = if stopped_for_upgrade && was_running {
                    execute_control(&record, super::dto::NginxControlAction::Start)
                        .is_ok_and(|output| output.success)
                        && wait_for_runtime_status(
                            &record,
                            NginxRuntimeStatus::Running,
                            Duration::from_secs(30),
                        ) == NginxRuntimeStatus::Running
                } else {
                    true
                };
                NginxUpgradeResult {
                    from_version,
                    target_version: input.target_version,
                    backup_id: backup_id.clone(),
                    success: false,
                    rolled_back: false,
                    rollback_succeeded: None,
                    error_code: Some(
                        if runtime_restored {
                            error.code
                        } else {
                            "NGINX_UPGRADE_RUNTIME_RESTORE_FAILED"
                        }
                        .to_owned(),
                    ),
                    resulting_status: detect_runtime_status(&record),
                }
            }
            Err(error) => {
                if let Some(path) = staging_directory.as_ref() {
                    let _ = fs::remove_dir_all(path);
                }
                *self.operation_phase.lock().unwrap() = None;
                self.emit_current_status(true);
                return Err(error);
            }
        };
        if let Some(path) = staging_directory.as_ref() {
            let _ = fs::remove_dir_all(path);
        }
        *self.operation_phase.lock().unwrap() = None;
        self.emit_current_status(true);
        final_result.resulting_status = self
            .get_record(&input.instance_id)
            .map(|current| detect_runtime_status(&current))
            .unwrap_or(NginxRuntimeStatus::Unknown);
        Ok(final_result)
    }

    fn set_upgrade_phase(
        &self,
        channel: &Channel<NginxUpgradeProgress>,
        phase: NginxOperationPhase,
        progress: u8,
        message_code: &str,
    ) {
        *self.operation_phase.lock().unwrap() = Some(phase);
        let _ = channel.send(NginxUpgradeProgress {
            phase,
            progress,
            message_code: message_code.to_owned(),
        });
        self.emit_current_status(false);
    }

    fn rollback_upgrade(
        &self,
        backup_parent: &Path,
        backup_id: &str,
        was_running: bool,
    ) -> NginxResult<()> {
        let original = backup_record(backup_parent, backup_id)?;
        let current_status = detect_runtime_status(&original);
        if current_status == NginxRuntimeStatus::Running {
            let _ = execute_control(&original, super::dto::NginxControlAction::Stop);
            let _ = wait_for_runtime_status(
                &original,
                NginxRuntimeStatus::Stopped,
                Duration::from_secs(30),
            );
        }
        restore_binary(backup_parent, backup_id, Path::new(&original.binary_path))?;
        {
            let mut registry = self.registry.lock().unwrap();
            registry.instance = Some(original.clone());
            registry.save(&self.registry_path)?;
        }
        if was_running {
            let output = execute_control(&original, super::dto::NginxControlAction::Start)?;
            if !output.success
                || wait_for_runtime_status(
                    &original,
                    NginxRuntimeStatus::Running,
                    Duration::from_secs(30),
                ) != NginxRuntimeStatus::Running
            {
                return Err(NginxError::new(
                    "NGINX_UPGRADE_ROLLBACK_FAILED",
                    "old nginx binary was restored but its runtime state was not recovered",
                ));
            }
        }
        Ok(())
    }

    pub fn subscribe_status(&self, channel: Channel<NginxStatusEvent>) -> NginxStatusSubscription {
        let subscription_id = self.next_subscription_id.fetch_add(1, Ordering::AcqRel);
        self.status_subscribers
            .lock()
            .unwrap()
            .insert(subscription_id, channel);
        self.ensure_monitor_thread();
        NginxStatusSubscription {
            subscription_id,
            initial_event: self.status_event(false),
        }
    }

    pub fn unsubscribe_status(&self, subscription_id: u64) {
        self.status_subscribers
            .lock()
            .unwrap()
            .remove(&subscription_id);
    }

    pub fn shutdown(&self) {
        self.monitor_shutdown.store(true, Ordering::Release);
        if let Some(handle) = self.monitor_handle.lock().unwrap().take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }

    pub fn list_system_services(&self) -> NginxResult<Vec<NginxSystemServiceCandidate>> {
        let discoveries = discover_system_services()?;
        let expires_at = Utc::now() + ChronoDuration::from_std(TOKEN_TTL).unwrap_or_default();
        let mut tokens = self.service_discoveries.lock().unwrap();
        tokens.clear();
        Ok(discoveries
            .into_iter()
            .map(|discovery| {
                let discovery_id = Uuid::new_v4().to_string();
                let candidate = NginxSystemServiceCandidate {
                    discovery_id: discovery_id.clone(),
                    display_name: discovery.display_name.clone(),
                    backend: discovery.backend,
                    domain: discovery.domain.clone(),
                    read_only: discovery.read_only,
                    expires_at: expires_at.to_rfc3339_opts(SecondsFormat::Secs, true),
                };
                tokens.insert(discovery_id, discovery);
                candidate
            })
            .collect())
    }

    pub fn inspect_system_service(
        &self,
        input: InspectNginxSystemServiceInput,
    ) -> NginxResult<NginxSystemServiceInspection> {
        let discovery = self
            .service_discoveries
            .lock()
            .unwrap()
            .get(&input.discovery_id)
            .filter(|token| token.expires_at > Instant::now())
            .cloned()
            .ok_or_else(expired_token_error)?;
        let instance = self.get_record(&input.instance_id)?;
        let matches = paths_identical(&discovery.binary, Path::new(&instance.binary_path));
        let inspection_id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + ChronoDuration::from_std(TOKEN_TTL).unwrap_or_default();
        self.service_inspections.lock().unwrap().insert(
            inspection_id.clone(),
            ServiceInspectionToken {
                instance_id: instance.id,
                discovery: discovery.clone(),
                expires_at: Instant::now() + TOKEN_TTL,
            },
        );
        Ok(NginxSystemServiceInspection {
            inspection_id,
            display_name: discovery.display_name,
            backend: discovery.backend,
            read_only: discovery.read_only,
            executable_matches: matches,
            expires_at: expires_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        })
    }

    pub fn register_system_service(
        &self,
        input: RegisterNginxSystemServiceInput,
    ) -> NginxResult<NginxInstance> {
        let inspection = self
            .service_inspections
            .lock()
            .unwrap()
            .remove(&input.inspection_id)
            .filter(|token| token.expires_at > Instant::now())
            .ok_or_else(expired_token_error)?;
        let mut registry = self.registry.lock().unwrap();
        let record = registry
            .instance
            .as_mut()
            .filter(|instance| instance.id == inspection.instance_id)
            .ok_or_else(instance_not_found)?;
        if !paths_identical(&inspection.discovery.binary, Path::new(&record.binary_path)) {
            return Err(NginxError::new(
                "NGINX_SERVICE_IDENTITY_MISMATCH",
                "service executable does not match the authorized nginx binary",
            ));
        }
        record.provider_identity = NginxProviderIdentity {
            provider: provider_name(inspection.discovery.backend).to_owned(),
            external_id: Some(inspection.discovery.external_id),
        };
        record.control_backend = inspection.discovery.backend;
        record.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
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
                    .instance
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

    fn get_record(&self, instance_id: &str) -> NginxResult<NginxInstanceRecord> {
        let registry = self.registry.lock().unwrap();
        if registry.migration_required() {
            return Err(migration_required());
        }
        registry
            .instance
            .as_ref()
            .filter(|instance| instance.id == instance_id)
            .cloned()
            .ok_or_else(instance_not_found)
    }

    fn bump_status_generation(&self) {
        self.status_generation.fetch_add(1, Ordering::AcqRel);
        self.status_sequence.store(0, Ordering::Release);
        self.emit_current_status(true);
    }

    fn status_event(&self, full_refresh: bool) -> NginxStatusEvent {
        build_status_event(
            &self.registry,
            &self.status_generation,
            &self.status_sequence,
            &self.operation_phase,
            &self.runtime_system,
            full_refresh,
        )
    }

    fn emit_current_status(&self, full_refresh: bool) {
        let event = self.status_event(full_refresh);
        self.status_subscribers
            .lock()
            .unwrap()
            .retain(|_, subscriber| subscriber.send(event.clone()).is_ok());
    }

    fn ensure_monitor_thread(&self) {
        let mut handle = self.monitor_handle.lock().unwrap();
        if handle.is_some() {
            return;
        }
        self.monitor_shutdown.store(false, Ordering::Release);
        let registry = Arc::clone(&self.registry);
        let subscribers = Arc::clone(&self.status_subscribers);
        let generation = Arc::clone(&self.status_generation);
        let sequence = Arc::clone(&self.status_sequence);
        let operation_phase = Arc::clone(&self.operation_phase);
        let runtime_system = Arc::clone(&self.runtime_system);
        let shutdown = Arc::clone(&self.monitor_shutdown);
        *handle = Some(thread::spawn(move || {
            let mut previous_signature = String::new();
            let mut cycle = 0_u64;
            while !shutdown.load(Ordering::Acquire) {
                let full_refresh = cycle.is_multiple_of(15);
                let event = build_status_event(
                    &registry,
                    &generation,
                    &sequence,
                    &operation_phase,
                    &runtime_system,
                    full_refresh,
                );
                let signature = status_signature(&event);
                if signature != previous_signature {
                    previous_signature = signature;
                    subscribers
                        .lock()
                        .unwrap()
                        .retain(|_, subscriber| subscriber.send(event.clone()).is_ok());
                }
                cycle = cycle.wrapping_add(1);
                thread::park_timeout(Duration::from_secs(2));
            }
        }));
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

fn build_status_event(
    registry: &Arc<Mutex<NginxRegistry>>,
    generation: &AtomicU64,
    sequence: &AtomicU64,
    operation_phase: &Arc<Mutex<Option<NginxOperationPhase>>>,
    runtime_system: &Arc<Mutex<System>>,
    full_refresh: bool,
) -> NginxStatusEvent {
    let instance = {
        let registry = registry.lock().unwrap();
        if registry.migration_required() {
            None
        } else {
            registry.instance.clone().map(|record| {
                if full_refresh {
                    refresh_instance(record)
                } else {
                    to_instance(record)
                }
            })
        }
    };
    let runtime_details = instance.as_ref().map(|value| {
        collect_runtime_details(
            &value.record,
            value.runtime_status,
            &mut runtime_system.lock().unwrap(),
        )
    });
    NginxStatusEvent {
        generation: generation.load(Ordering::Acquire),
        sequence: sequence.fetch_add(1, Ordering::AcqRel) + 1,
        observed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        instance,
        runtime_details,
        operation_phase: *operation_phase.lock().unwrap(),
    }
}

fn status_signature(event: &NginxStatusEvent) -> String {
    let instance = event.instance.as_ref();
    format!(
        "{}|{}|{:?}|{:?}|{:?}|{}|{:?}|{:?}",
        event.generation,
        instance
            .map(|value| value.record.id.as_str())
            .unwrap_or_default(),
        instance.map(|value| value.record.lifecycle_state),
        instance.map(|value| value.runtime_status),
        event.operation_phase,
        instance
            .map(|value| value.record.version.as_str())
            .unwrap_or_default(),
        event
            .runtime_details
            .as_ref()
            .map(|value| value.total_cpu_usage),
        event
            .runtime_details
            .as_ref()
            .map(|value| value.total_memory_bytes),
    )
}

fn collect_runtime_details(
    record: &NginxInstanceRecord,
    status: NginxRuntimeStatus,
    system: &mut System,
) -> NginxRuntimeDetails {
    let master_pid = verified_master_pid(record);
    system.refresh_processes(ProcessesToUpdate::All, true);
    let expected = Path::new(&record.binary_path);
    let mut processes = system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let executable = process.exe()?;
            if !paths_identical(executable, expected) {
                return None;
            }
            let started_at = DateTime::<Utc>::from_timestamp(process.start_time() as i64, 0)
                .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true));
            Some(NginxRuntimeProcess {
                pid: pid.as_u32(),
                parent_pid: process.parent().map(|value| value.as_u32()),
                role: if master_pid.is_some_and(|master| master == *pid) {
                    NginxProcessRole::Master
                } else {
                    NginxProcessRole::Worker
                },
                cpu_usage: process.cpu_usage(),
                memory_bytes: process.memory(),
                started_at,
                uptime_seconds: process.run_time(),
                executable_verified: true,
            })
        })
        .collect::<Vec<_>>();
    processes.sort_by_key(|process| (process.role != NginxProcessRole::Master, process.pid));
    let started_at = processes
        .iter()
        .filter_map(|process| process.started_at.clone())
        .min();
    let uptime_seconds = processes.iter().map(|process| process.uptime_seconds).max();
    NginxRuntimeDetails {
        instance_id: record.id.clone(),
        observed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        status,
        master_pid: master_pid.map(|value| value.as_u32()),
        worker_count: processes
            .iter()
            .filter(|process| process.role == NginxProcessRole::Worker)
            .count(),
        total_cpu_usage: processes.iter().map(|process| process.cpu_usage).sum(),
        total_memory_bytes: processes.iter().map(|process| process.memory_bytes).sum(),
        started_at,
        uptime_seconds,
        processes,
        process_metrics: NginxRuntimeMetricAvailability::Available,
        listeners: Vec::new(),
        listener_metrics: NginxRuntimeMetricAvailability::Unavailable,
        connection_metrics: NginxRuntimeMetricAvailability::Unavailable,
    }
}

fn verified_master_pid(record: &NginxInstanceRecord) -> Option<Pid> {
    if !matches!(
        record.control_backend,
        NginxControlBackend::Portable | NginxControlBackend::None
    ) {
        return None;
    }
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
    let canonical = pid_path.canonicalize().ok()?;
    if !record
        .authorized_roots
        .iter()
        .any(|root| canonical.starts_with(Path::new(root)))
        || reject_reparse_points(&canonical).is_err()
        || fs::metadata(&canonical).ok()?.len() > 64
    {
        return None;
    }
    let pid = fs::read_to_string(canonical)
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()?;
    Some(Pid::from(pid))
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

fn validate_candidate_configuration(
    candidate_binary: &Path,
    record: &NginxInstanceRecord,
) -> NginxResult<()> {
    let root = Path::new(&record.root_path);
    let root_argument = record.root_path.clone();
    let config_argument = record
        .config_path
        .clone()
        .unwrap_or_else(|| "conf/nginx.conf".to_owned());
    let output = run_command(
        candidate_binary,
        &["-t", "-p", &root_argument, "-c", &config_argument],
        Some(root),
        15,
    )?;
    if output.success {
        Ok(())
    } else {
        Err(NginxError::new(
            "NGINX_UPGRADE_CONFIG_PRECHECK_FAILED",
            "candidate nginx.exe rejected the current configuration",
        ))
    }
}

fn validate_installed_upgrade(
    record: &NginxInstanceRecord,
    target_version: &str,
) -> NginxResult<()> {
    let binary = Path::new(&record.binary_path);
    let output = run_nginx(binary, &["-V"])?;
    let version = parse_version(&format!("{}\n{}", output.stdout, output.stderr));
    if !output.success || version.as_deref() != Some(target_version) {
        return Err(NginxError::new(
            "NGINX_UPGRADE_POSTCHECK_FAILED",
            "installed nginx.exe did not report the target version",
        ));
    }
    validate_candidate_configuration(binary, record)
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
            can_control: available
                && record.control_backend != NginxControlBackend::None
                && record.control_backend != NginxControlBackend::LaunchDaemonReadOnly,
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
    match record.control_backend {
        NginxControlBackend::WindowsScm => return detect_windows_service_status(record),
        NginxControlBackend::Systemd => return detect_systemd_status(record),
        NginxControlBackend::LaunchAgent | NginxControlBackend::LaunchDaemonReadOnly => {
            return detect_launchd_status(record)
        }
        NginxControlBackend::Portable | NginxControlBackend::None => {}
    }
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
        return unverified_process_status(record);
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
        return unverified_process_status(record);
    }
    let Some(executable) = system.process(pid).and_then(|process| process.exe()) else {
        return NginxRuntimeStatus::Unknown;
    };
    match executable_identity_status(executable, Path::new(&record.binary_path)) {
        NginxRuntimeStatus::Running => NginxRuntimeStatus::Running,
        _ => unverified_process_status(record),
    }
}

#[cfg(windows)]
fn detect_windows_service_status(record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    let Some(service) = record.provider_identity.external_id.as_deref() else {
        return NginxRuntimeStatus::Unknown;
    };
    match run_command(Path::new("sc.exe"), &["query", service], None, 10) {
        Ok(output) if output.success && output.stdout.contains("RUNNING") => {
            NginxRuntimeStatus::Running
        }
        Ok(output) if output.success && output.stdout.contains("STOPPED") => {
            NginxRuntimeStatus::Stopped
        }
        _ => NginxRuntimeStatus::Unknown,
    }
}

#[cfg(not(windows))]
fn detect_windows_service_status(_record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    NginxRuntimeStatus::Unknown
}

#[cfg(target_os = "linux")]
fn detect_systemd_status(record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    let Some(unit) = record.provider_identity.external_id.as_deref() else {
        return NginxRuntimeStatus::Unknown;
    };
    match run_command(
        Path::new("/usr/bin/systemctl"),
        &["is-active", unit],
        None,
        10,
    ) {
        Ok(output) if output.stdout.trim() == "active" => NginxRuntimeStatus::Running,
        Ok(output) if matches!(output.stdout.trim(), "inactive" | "failed" | "deactivating") => {
            NginxRuntimeStatus::Stopped
        }
        _ => NginxRuntimeStatus::Unknown,
    }
}

#[cfg(not(target_os = "linux"))]
fn detect_systemd_status(_record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    NginxRuntimeStatus::Unknown
}

#[cfg(target_os = "macos")]
fn detect_launchd_status(record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    let Some(label) = record.provider_identity.external_id.as_deref() else {
        return NginxRuntimeStatus::Unknown;
    };
    let domain = if record.control_backend == NginxControlBackend::LaunchDaemonReadOnly {
        format!("system/{label}")
    } else {
        let Ok(uid) = run_command(Path::new("/usr/bin/id"), &["-u"], None, 5) else {
            return NginxRuntimeStatus::Unknown;
        };
        format!("gui/{}/{label}", uid.stdout.trim())
    };
    match run_command(Path::new("/bin/launchctl"), &["print", &domain], None, 10) {
        Ok(output) if output.success => NginxRuntimeStatus::Running,
        Ok(_) => NginxRuntimeStatus::Stopped,
        Err(_) => NginxRuntimeStatus::Unknown,
    }
}

#[cfg(not(target_os = "macos"))]
fn detect_launchd_status(_record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    NginxRuntimeStatus::Unknown
}

fn unverified_process_status(record: &NginxInstanceRecord) -> NginxRuntimeStatus {
    use sysinfo::{ProcessesToUpdate, System};

    let expected = Path::new(&record.binary_path);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    if system
        .processes()
        .values()
        .filter_map(|process| process.exe())
        .any(|actual| paths_identical(actual, expected))
    {
        NginxRuntimeStatus::Conflict
    } else {
        NginxRuntimeStatus::Stopped
    }
}

fn wait_for_runtime_status(
    record: &NginxInstanceRecord,
    target: NginxRuntimeStatus,
    timeout: Duration,
) -> NginxRuntimeStatus {
    let deadline = Instant::now() + timeout;
    loop {
        let status = detect_runtime_status(record);
        if status == target || Instant::now() >= deadline {
            return status;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn control_outcome(
    action: super::dto::NginxControlAction,
    status: NginxRuntimeStatus,
) -> NginxResult<NginxOperationOutcome> {
    match (action, status) {
        (super::dto::NginxControlAction::Start, NginxRuntimeStatus::Running)
        | (super::dto::NginxControlAction::Stop, NginxRuntimeStatus::Stopped) => {
            Ok(NginxOperationOutcome::Noop)
        }
        (super::dto::NginxControlAction::Start, NginxRuntimeStatus::Stopped)
        | (super::dto::NginxControlAction::Stop, NginxRuntimeStatus::Running)
        | (super::dto::NginxControlAction::Reload, NginxRuntimeStatus::Running)
        | (super::dto::NginxControlAction::Restart, NginxRuntimeStatus::Running) => {
            Ok(NginxOperationOutcome::Executed)
        }
        (_, NginxRuntimeStatus::Conflict) => Err(NginxError::new(
            "NGINX_RUNTIME_CONFLICT",
            "nginx processes exist but the registered master process cannot be verified",
        )),
        _ => Err(NginxError::new(
            "NGINX_RUNTIME_STATE_INVALID",
            "the action is not valid for the current nginx runtime state",
        )),
    }
}

fn executable_identity_status(actual: &Path, expected: &Path) -> NginxRuntimeStatus {
    match (actual.canonicalize(), expected.canonicalize()) {
        (Ok(actual), Ok(expected)) if actual == expected => NginxRuntimeStatus::Running,
        (Ok(_), Ok(_)) => NginxRuntimeStatus::Stopped,
        _ => NginxRuntimeStatus::Unknown,
    }
}

fn paths_identical(actual: &Path, expected: &Path) -> bool {
    match (actual.canonicalize(), expected.canonicalize()) {
        (Ok(actual), Ok(expected)) if cfg!(windows) => actual
            .to_string_lossy()
            .eq_ignore_ascii_case(&expected.to_string_lossy()),
        (Ok(actual), Ok(expected)) => actual == expected,
        _ => false,
    }
}

fn provider_name(backend: NginxControlBackend) -> &'static str {
    match backend {
        NginxControlBackend::Portable => "portable",
        NginxControlBackend::WindowsScm => "windows-scm",
        NginxControlBackend::Systemd => "systemd",
        NginxControlBackend::LaunchAgent => "launch-agent",
        NginxControlBackend::LaunchDaemonReadOnly => "launch-daemon",
        NginxControlBackend::None => "none",
    }
}

#[cfg(windows)]
fn discover_system_services() -> NginxResult<Vec<ServiceDiscoveryToken>> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let services = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SYSTEM\\CurrentControlSet\\Services")
        .map_err(|error| NginxError::io("open Windows service registry", error))?;
    let mut result = Vec::new();
    for name in services.enum_keys().flatten() {
        let Ok(service) = services.open_subkey(&name) else {
            continue;
        };
        let Ok(image_path) = service.get_value::<String, _>("ImagePath") else {
            continue;
        };
        let Some(binary) = windows_service_binary(&image_path) else {
            continue;
        };
        if binary
            .file_name()
            .and_then(|value| value.to_str())
            .is_none_or(|value| !value.eq_ignore_ascii_case("nginx.exe"))
        {
            continue;
        }
        let Ok(binary) = binary.canonicalize() else {
            continue;
        };
        result.push(ServiceDiscoveryToken {
            display_name: name.clone(),
            external_id: name,
            binary,
            backend: NginxControlBackend::WindowsScm,
            domain: "system".to_owned(),
            read_only: false,
            expires_at: Instant::now() + TOKEN_TTL,
        });
    }
    Ok(result)
}

#[cfg(windows)]
fn windows_service_binary(command_line: &str) -> Option<PathBuf> {
    let value = command_line.trim();
    let executable = if let Some(value) = value.strip_prefix('"') {
        value.split_once('"')?.0
    } else {
        value.split_whitespace().next()?
    };
    if executable.contains('%') || executable.is_empty() {
        None
    } else {
        Some(PathBuf::from(executable))
    }
}

#[cfg(target_os = "linux")]
fn discover_system_services() -> NginxResult<Vec<ServiceDiscoveryToken>> {
    let mut result = Vec::new();
    for (root, domain) in [
        (PathBuf::from("/etc/systemd/system"), "system"),
        (PathBuf::from("/usr/lib/systemd/system"), "system"),
    ] {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("service") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Some(executable) = text.lines().find_map(|line| {
                line.trim()
                    .strip_prefix("ExecStart=")
                    .and_then(|value| value.split_whitespace().next())
            }) else {
                continue;
            };
            let binary = PathBuf::from(executable.trim_start_matches(['-', '+', '!']));
            if binary.file_name().and_then(|value| value.to_str()) != Some("nginx") {
                continue;
            }
            let Ok(binary) = binary.canonicalize() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            result.push(ServiceDiscoveryToken {
                display_name: name.clone(),
                external_id: name,
                binary,
                backend: NginxControlBackend::Systemd,
                domain: domain.to_owned(),
                read_only: false,
                expires_at: Instant::now() + TOKEN_TTL,
            });
        }
    }
    Ok(result)
}

#[cfg(target_os = "macos")]
fn discover_system_services() -> NginxResult<Vec<ServiceDiscoveryToken>> {
    use plist::Value;

    let mut result = Vec::new();
    let user_agents = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/LaunchAgents"));
    let roots = user_agents
        .into_iter()
        .map(|path| (path, "user", false, NginxControlBackend::LaunchAgent))
        .chain(std::iter::once((
            PathBuf::from("/Library/LaunchDaemons"),
            "system",
            true,
            NginxControlBackend::LaunchDaemonReadOnly,
        )));
    for (root, domain, read_only, backend) in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("plist") {
                continue;
            }
            let Ok(Value::Dictionary(dictionary)) = Value::from_file(&path) else {
                continue;
            };
            let label = dictionary.get("Label").and_then(Value::as_string);
            let binary = dictionary
                .get("Program")
                .and_then(Value::as_string)
                .or_else(|| {
                    dictionary
                        .get("ProgramArguments")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .and_then(Value::as_string)
                });
            let (Some(label), Some(binary)) = (label, binary) else {
                continue;
            };
            let Ok(binary) = PathBuf::from(binary).canonicalize() else {
                continue;
            };
            if binary.file_name().and_then(|value| value.to_str()) != Some("nginx") {
                continue;
            }
            result.push(ServiceDiscoveryToken {
                display_name: label.to_owned(),
                external_id: label.to_owned(),
                binary,
                backend,
                domain: domain.to_owned(),
                read_only,
                expires_at: Instant::now() + TOKEN_TTL,
            });
        }
    }
    Ok(result)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn discover_system_services() -> NginxResult<Vec<ServiceDiscoveryToken>> {
    Ok(Vec::new())
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

fn migration_required() -> NginxError {
    NginxError::new(
        "NGINX_REGISTRY_MIGRATION_REQUIRED",
        "resolve the legacy multi-instance registry first",
    )
}

pub(crate) fn reject_reparse_points(path: &Path) -> NginxResult<()> {
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
    fn control_matrix_is_idempotent_and_rejects_unverified_states() {
        use super::super::dto::NginxControlAction;

        assert_eq!(
            control_outcome(NginxControlAction::Start, NginxRuntimeStatus::Running).unwrap(),
            NginxOperationOutcome::Noop
        );
        assert_eq!(
            control_outcome(NginxControlAction::Stop, NginxRuntimeStatus::Stopped).unwrap(),
            NginxOperationOutcome::Noop
        );
        assert_eq!(
            control_outcome(NginxControlAction::Restart, NginxRuntimeStatus::Running).unwrap(),
            NginxOperationOutcome::Executed
        );
        assert_eq!(
            control_outcome(NginxControlAction::Start, NginxRuntimeStatus::Conflict)
                .unwrap_err()
                .code,
            "NGINX_RUNTIME_CONFLICT"
        );
        assert_eq!(
            control_outcome(NginxControlAction::Reload, NginxRuntimeStatus::Unknown)
                .unwrap_err()
                .code,
            "NGINX_RUNTIME_STATE_INVALID"
        );
    }

    #[test]
    fn operation_guard_never_queues_a_second_operation() {
        let flag = Arc::new(AtomicBool::new(false));
        let first = OperationGuard::try_acquire(Arc::clone(&flag)).unwrap();
        assert_eq!(
            OperationGuard::try_acquire(Arc::clone(&flag))
                .unwrap_err()
                .code,
            "NGINX_OPERATION_IN_PROGRESS"
        );
        drop(first);
        assert!(OperationGuard::try_acquire(flag).is_ok());
    }

    #[test]
    fn status_subscription_has_initial_sequence_and_explicit_cleanup() {
        let directory = tempfile::tempdir().expect("tempdir");
        let manager = NginxManager::new(directory.path().to_path_buf()).expect("manager");
        let received = Arc::new(AtomicU64::new(0));
        let received_by_channel = Arc::clone(&received);
        let subscription = manager.subscribe_status(Channel::new(move |_| {
            received_by_channel.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }));
        assert!(subscription.initial_event.sequence > 0);
        assert!(subscription.initial_event.instance.is_none());
        manager.unsubscribe_status(subscription.subscription_id);
        let before = received.load(Ordering::Acquire);
        manager.emit_current_status(false);
        assert_eq!(received.load(Ordering::Acquire), before);
        manager.shutdown();
    }

    #[test]
    fn status_signature_deduplicates_observation_only_changes() {
        let directory = tempfile::tempdir().expect("tempdir");
        let manager = NginxManager::new(directory.path().to_path_buf()).expect("manager");
        let first = manager.status_event(false);
        let second = manager.status_event(false);
        assert!(second.sequence > first.sequence);
        assert_eq!(status_signature(&first), status_signature(&second));
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

    #[test]
    fn runtime_details_never_infer_listener_or_connection_metrics() {
        let directory = tempfile::tempdir().expect("tempdir");
        let binary = directory
            .path()
            .join(if cfg!(windows) { "nginx.exe" } else { "nginx" });
        fs::write(&binary, b"fixture").expect("write fixture");
        let fixture = record(
            directory.path(),
            &binary,
            fingerprint(&binary).expect("fingerprint"),
        );
        let details =
            collect_runtime_details(&fixture, NginxRuntimeStatus::Stopped, &mut System::new());

        assert!(details.listeners.is_empty());
        assert_eq!(
            details.listener_metrics,
            NginxRuntimeMetricAvailability::Unavailable
        );
        assert_eq!(
            details.connection_metrics,
            NginxRuntimeMetricAvailability::Unavailable
        );
    }
}
