use super::config_graph::build_config_graph;
use super::configuration::load_configuration;
use super::control::execute as execute_control;
use super::dto::{
    NginxConfigGraph, NginxConfigRevision, NginxConfiguration, NginxControlAction,
    NginxGlobalConfigApplyMode, NginxGlobalConfigApplyResult, NginxGlobalConfigFieldError,
    NginxGlobalConfigPatchValidation, NginxGlobalConfiguration, NginxGlobalConfigurationPatch,
    NginxInstanceRecord,
};
use super::error::{NginxError, NginxResult};
use super::process::run_command;
use super::registry::{write_bytes_atomically, write_json_atomically};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};
use uuid::Uuid;

const MAX_AUDIT_EVENTS: usize = 1_000;

#[derive(Clone, Debug)]
struct TextPatch {
    start: usize,
    end: usize,
    replacement: String,
}

struct TransactionOutcome {
    success: bool,
    rolled_back: bool,
    rollback_succeeded: Option<bool>,
    error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigAuditEvent {
    id: String,
    instance_id: String,
    occurred_at: String,
    mode: NginxGlobalConfigApplyMode,
    success: bool,
    rolled_back: bool,
    error_code: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigAuditFile {
    version: u8,
    events: Vec<ConfigAuditEvent>,
}

pub fn read_global_configuration(
    configuration: &NginxConfiguration,
) -> NginxResult<NginxGlobalConfiguration> {
    let graph = build_config_graph(configuration)?;
    let source = entry_source(configuration)?;
    let events = singleton_directive(&source.directives, "events")?;
    Ok(NginxGlobalConfiguration {
        instance_id: configuration.instance_id.clone(),
        revision: graph.revision,
        values: NginxGlobalConfigurationPatch {
            worker_processes: directive_value(&source.directives, "worker_processes")?,
            worker_rlimit_nofile: directive_value(&source.directives, "worker_rlimit_nofile")?,
            pid: directive_value(&source.directives, "pid")?,
            error_log: directive_value(&source.directives, "error_log")?,
            top_level_includes: source
                .directives
                .iter()
                .filter(|directive| directive.name == "include")
                .map(|directive| directive.arguments.join(" "))
                .collect(),
            worker_connections: events
                .map(|events| directive_value(&events.children, "worker_connections"))
                .transpose()?
                .flatten(),
            multi_accept: events
                .map(|events| directive_value(&events.children, "multi_accept"))
                .transpose()?
                .flatten(),
            accept_mutex: events
                .map(|events| directive_value(&events.children, "accept_mutex"))
                .transpose()?
                .flatten(),
            accept_mutex_delay: events
                .map(|events| directive_value(&events.children, "accept_mutex_delay"))
                .transpose()?
                .flatten(),
        },
    })
}

pub fn validate_global_patch(
    record: &NginxInstanceRecord,
    expected_revision: &str,
    patch: &NginxGlobalConfigurationPatch,
) -> NginxResult<NginxGlobalConfigPatchValidation> {
    let configuration = load_configuration(record)?;
    let current_graph = build_config_graph(&configuration)?;
    ensure_revision(&current_graph, expected_revision)?;
    let (bytes, field_errors) = build_patched_bytes(&configuration, patch)?;
    if !field_errors.is_empty() {
        return Ok(NginxGlobalConfigPatchValidation {
            current_revision: current_graph.revision,
            proposed_revision: None,
            field_errors,
            parser_valid: false,
            native_valid: false,
            native_error_code: None,
        });
    }
    validate_candidate(record, current_graph.revision, &bytes)
}

pub fn apply_global_patch(
    record: &NginxInstanceRecord,
    data_directory: &Path,
    expected_revision: &str,
    patch: &NginxGlobalConfigurationPatch,
    mode: NginxGlobalConfigApplyMode,
) -> NginxResult<NginxGlobalConfigApplyResult> {
    let configuration = load_configuration(record)?;
    let current_graph = build_config_graph(&configuration)?;
    ensure_revision(&current_graph, expected_revision)?;
    let (bytes, field_errors) = build_patched_bytes(&configuration, patch)?;
    if !field_errors.is_empty() {
        return Err(NginxError::new(
            "NGINX_CONFIG_PATCH_INVALID",
            "one or more global configuration fields are invalid",
        ));
    }
    let original_revision = current_graph.revision.clone();
    let validation = validate_candidate(record, current_graph.revision, &bytes)?;
    if !validation.parser_valid || !validation.native_valid {
        let code = if validation.parser_valid {
            "NGINX_CONFIG_NATIVE_VALIDATION_FAILED"
        } else {
            "NGINX_CONFIG_PATCH_INVALID"
        };
        return Err(NginxError::new(
            code,
            "the proposed configuration did not pass validation",
        ));
    }
    let target = Path::new(record.config_path.as_deref().ok_or_else(config_not_found)?);
    let original =
        fs::read(target).map_err(|error| NginxError::io("read configuration snapshot", error))?;
    let snapshot = data_directory
        .join("configuration-snapshots")
        .join(format!("{}.conf", Uuid::new_v4()));
    write_bytes_atomically(&snapshot, &original, "config-snapshot")?;
    let outcome = execute_config_transaction(
        target,
        &bytes,
        &original,
        || match mode {
            NginxGlobalConfigApplyMode::Save => Ok(()),
            NginxGlobalConfigApplyMode::Reload => {
                execute_control(record, NginxControlAction::Reload)
                    .and_then(require_process_success)
            }
            NginxGlobalConfigApplyMode::Restart => {
                execute_control(record, NginxControlAction::Restart)
                    .and_then(require_process_success)
            }
        },
        || validate_and_restore_runtime(record, target, mode),
    )?;
    let result = if outcome.success {
        NginxGlobalConfigApplyResult {
            revision: build_config_graph(&load_configuration(record)?)?.revision,
            mode,
            success: true,
            rolled_back: false,
            rollback_succeeded: None,
            error_code: None,
        }
    } else {
        NginxGlobalConfigApplyResult {
            revision: load_configuration(record)
                .and_then(|configuration| build_config_graph(&configuration))
                .map(|graph| graph.revision)
                .unwrap_or(original_revision),
            mode,
            success: false,
            rolled_back: outcome.rolled_back,
            rollback_succeeded: outcome.rollback_succeeded,
            error_code: outcome.error_code,
        }
    };
    let _ = fs::remove_file(snapshot);
    record_audit(data_directory, record, &result)?;
    Ok(result)
}

fn build_patched_bytes(
    configuration: &NginxConfiguration,
    patch: &NginxGlobalConfigurationPatch,
) -> NginxResult<(Vec<u8>, Vec<NginxGlobalConfigFieldError>)> {
    let source = entry_source(configuration)?;
    let field_errors = validate_fields(patch);
    if !field_errors.is_empty() {
        return Ok((Vec::new(), field_errors));
    }
    let mut patches = Vec::new();
    let mut top_insertions = Vec::new();
    patch_single(
        &source.directives,
        "worker_processes",
        patch.worker_processes.as_deref(),
        &mut patches,
        &mut top_insertions,
    )?;
    patch_single(
        &source.directives,
        "worker_rlimit_nofile",
        patch.worker_rlimit_nofile.as_deref(),
        &mut patches,
        &mut top_insertions,
    )?;
    patch_single(
        &source.directives,
        "pid",
        patch.pid.as_deref(),
        &mut patches,
        &mut top_insertions,
    )?;
    patch_single(
        &source.directives,
        "error_log",
        patch.error_log.as_deref(),
        &mut patches,
        &mut top_insertions,
    )?;
    patch_includes(
        &source.directives,
        &patch.top_level_includes,
        &mut patches,
        &mut top_insertions,
    );
    let events = singleton_directive(&source.directives, "events")?;
    let event_values = [
        ("worker_connections", patch.worker_connections.as_deref()),
        ("multi_accept", patch.multi_accept.as_deref()),
        ("accept_mutex", patch.accept_mutex.as_deref()),
        ("accept_mutex_delay", patch.accept_mutex_delay.as_deref()),
    ];
    let needs_events = event_values.iter().any(|(_, value)| value.is_some());
    if needs_events && events.is_none() {
        return Err(NginxError::new(
            "NGINX_CONFIG_EVENTS_BLOCK_REQUIRED",
            "events fields cannot be added without an entry-file events block",
        ));
    }
    let mut event_insertions = Vec::new();
    if let Some(events) = events {
        for (name, value) in event_values {
            patch_single(
                &events.children,
                name,
                value,
                &mut patches,
                &mut event_insertions,
            )?;
        }
        if !event_insertions.is_empty() {
            patches.push(TextPatch {
                start: events.location.byte_end.saturating_sub(1),
                end: events.location.byte_end.saturating_sub(1),
                replacement: format!("\n    {}", event_insertions.join("    ")),
            });
        }
    }
    if !top_insertions.is_empty() {
        patches.push(TextPatch {
            start: 0,
            end: 0,
            replacement: top_insertions.join(""),
        });
    }
    apply_text_patches(&source.text, patches).map(|text| (text.into_bytes(), Vec::new()))
}

fn patch_single(
    directives: &[super::dto::NginxDirective],
    name: &str,
    value: Option<&str>,
    patches: &mut Vec<TextPatch>,
    insertions: &mut Vec<String>,
) -> NginxResult<()> {
    let matches = directives
        .iter()
        .filter(|directive| directive.name == name)
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(NginxError::new(
            "NGINX_CONFIG_FIELD_AMBIGUOUS",
            "an editable singleton directive appears more than once",
        ));
    }
    match (matches.first(), value) {
        (Some(directive), Some(value)) if directive.arguments.join(" ") != value => {
            patches.push(TextPatch {
                start: directive.location.byte_start,
                end: directive.location.byte_end,
                replacement: format!("{name} {value};"),
            });
        }
        (Some(directive), None) => patches.push(TextPatch {
            start: directive.location.byte_start,
            end: directive.location.byte_end,
            replacement: String::new(),
        }),
        (None, Some(value)) => insertions.push(format!("{name} {value};\n")),
        _ => {}
    }
    Ok(())
}

fn patch_includes(
    directives: &[super::dto::NginxDirective],
    values: &[String],
    patches: &mut Vec<TextPatch>,
    insertions: &mut Vec<String>,
) {
    let existing = directives
        .iter()
        .filter(|directive| directive.name == "include")
        .collect::<Vec<_>>();
    for (index, directive) in existing.iter().enumerate() {
        let replacement = values
            .get(index)
            .map(|value| format!("include {value};"))
            .unwrap_or_default();
        if directive.arguments.join(" ") != values.get(index).cloned().unwrap_or_default() {
            patches.push(TextPatch {
                start: directive.location.byte_start,
                end: directive.location.byte_end,
                replacement,
            });
        }
    }
    for value in values.iter().skip(existing.len()) {
        insertions.push(format!("include {value};\n"));
    }
}

fn validate_fields(patch: &NginxGlobalConfigurationPatch) -> Vec<NginxGlobalConfigFieldError> {
    let mut errors = Vec::new();
    validate_number_or_auto(
        "workerProcesses",
        patch.worker_processes.as_deref(),
        1,
        1_024,
        &mut errors,
    );
    validate_number(
        "workerRlimitNofile",
        patch.worker_rlimit_nofile.as_deref(),
        1,
        10_000_000,
        &mut errors,
    );
    validate_number(
        "workerConnections",
        patch.worker_connections.as_deref(),
        1,
        10_000_000,
        &mut errors,
    );
    validate_choice(
        "multiAccept",
        patch.multi_accept.as_deref(),
        &["on", "off"],
        &mut errors,
    );
    validate_choice(
        "acceptMutex",
        patch.accept_mutex.as_deref(),
        &["on", "off"],
        &mut errors,
    );
    if patch
        .accept_mutex_delay
        .as_deref()
        .is_some_and(|value| !valid_duration(value))
    {
        field_error(
            &mut errors,
            "acceptMutexDelay",
            "NGINX_CONFIG_VALUE_INVALID",
        );
    }
    if patch
        .pid
        .as_deref()
        .is_some_and(|value| !valid_relative_path(value))
    {
        field_error(&mut errors, "pid", "NGINX_CONFIG_PATH_INVALID");
    }
    if patch
        .error_log
        .as_deref()
        .is_some_and(|value| !valid_error_log(value))
    {
        field_error(&mut errors, "errorLog", "NGINX_CONFIG_PATH_INVALID");
    }
    for value in &patch.top_level_includes {
        if !valid_relative_path(value) {
            field_error(&mut errors, "topLevelIncludes", "NGINX_CONFIG_PATH_INVALID");
            break;
        }
    }
    errors
}

fn validate_number_or_auto(
    field: &str,
    value: Option<&str>,
    min: u64,
    max: u64,
    errors: &mut Vec<NginxGlobalConfigFieldError>,
) {
    if value == Some("auto") {
        return;
    }
    validate_number(field, value, min, max, errors);
}

fn validate_number(
    field: &str,
    value: Option<&str>,
    min: u64,
    max: u64,
    errors: &mut Vec<NginxGlobalConfigFieldError>,
) {
    if value.is_some_and(|value| {
        value
            .parse::<u64>()
            .map_or(true, |number| !(min..=max).contains(&number))
    }) {
        field_error(errors, field, "NGINX_CONFIG_VALUE_INVALID");
    }
}

fn validate_choice(
    field: &str,
    value: Option<&str>,
    choices: &[&str],
    errors: &mut Vec<NginxGlobalConfigFieldError>,
) {
    if value.is_some_and(|value| !choices.contains(&value)) {
        field_error(errors, field, "NGINX_CONFIG_VALUE_INVALID");
    }
}

fn valid_duration(value: &str) -> bool {
    let split = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());
    split > 0
        && value[..split].parse::<u64>().is_ok()
        && matches!(&value[split..], "ms" | "s" | "m")
}

fn valid_error_log(value: &str) -> bool {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    matches!(parts.len(), 1 | 2)
        && (parts[0] == "stderr" || valid_relative_path(parts[0]))
        && parts.get(1).is_none_or(|level| {
            matches!(
                *level,
                "debug" | "info" | "notice" | "warn" | "error" | "crit" | "alert" | "emerg"
            )
        })
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && !value
            .chars()
            .any(|character| matches!(character, '\0' | '\r' | '\n' | ';' | '{' | '}' | '#'))
        && {
            let path = Path::new(value);
            !path.is_absolute()
                && path
                    .components()
                    .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        }
}

fn field_error(errors: &mut Vec<NginxGlobalConfigFieldError>, field: &str, code: &str) {
    errors.push(NginxGlobalConfigFieldError {
        field: field.to_owned(),
        code: code.to_owned(),
    });
}

fn validate_candidate(
    record: &NginxInstanceRecord,
    current_revision: NginxConfigRevision,
    bytes: &[u8],
) -> NginxResult<NginxGlobalConfigPatchValidation> {
    let target = Path::new(record.config_path.as_deref().ok_or_else(config_not_found)?);
    let parent = target.parent().ok_or_else(config_not_found)?;
    let candidate = parent.join(format!(".zx-manager-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&candidate)
            .map_err(|error| NginxError::io("create configuration candidate", error))?;
        if let Ok(metadata) = fs::metadata(target) {
            fs::set_permissions(&candidate, metadata.permissions())
                .map_err(|error| NginxError::io("preserve configuration permissions", error))?;
        }
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| NginxError::io("write configuration candidate", error))?;
        let mut candidate_record = record.clone();
        candidate_record.config_path = Some(candidate.to_string_lossy().into_owned());
        let parsed = load_configuration(&candidate_record);
        let (parser_valid, proposed_revision) = match parsed {
            Ok(configuration) => {
                let graph = build_config_graph(&configuration)?;
                let valid = !graph
                    .diagnostics
                    .iter()
                    .any(|item| item.severity == "error");
                (valid, Some(graph.revision))
            }
            Err(_) => (false, None),
        };
        let output = run_command(
            Path::new(&record.binary_path),
            &[
                "-t",
                "-p",
                &record.root_path,
                "-c",
                candidate.to_string_lossy().as_ref(),
            ],
            Some(Path::new(&record.root_path)),
            15,
        );
        let (native_valid, native_error_code) = match output {
            Ok(output) if output.success => (true, None),
            Ok(_) => (
                false,
                Some("NGINX_CONFIG_NATIVE_VALIDATION_FAILED".to_owned()),
            ),
            Err(error) => (false, Some(error.code.to_owned())),
        };
        Ok(NginxGlobalConfigPatchValidation {
            current_revision,
            proposed_revision,
            field_errors: Vec::new(),
            parser_valid,
            native_valid,
            native_error_code,
        })
    })();
    let _ = fs::remove_file(candidate);
    result
}

fn execute_config_transaction(
    target: &Path,
    candidate: &[u8],
    original: &[u8],
    action: impl FnOnce() -> NginxResult<()>,
    rollback_action: impl FnOnce() -> NginxResult<()>,
) -> NginxResult<TransactionOutcome> {
    write_bytes_atomically(target, candidate, "config-apply")?;
    let Err(action_error) = action() else {
        return Ok(TransactionOutcome {
            success: true,
            rolled_back: false,
            rollback_succeeded: None,
            error_code: None,
        });
    };
    let rollback = write_bytes_atomically(target, original, "config-rollback")
        .and_then(|()| rollback_action());
    let rollback_succeeded = rollback.is_ok();
    Ok(TransactionOutcome {
        success: false,
        rolled_back: true,
        rollback_succeeded: Some(rollback_succeeded),
        error_code: Some(if rollback_succeeded {
            action_error.code.to_owned()
        } else {
            "NGINX_CONFIG_ROLLBACK_CRITICAL".to_owned()
        }),
    })
}

fn validate_and_restore_runtime(
    record: &NginxInstanceRecord,
    target: &Path,
    mode: NginxGlobalConfigApplyMode,
) -> NginxResult<()> {
    let validation = run_command(
        Path::new(&record.binary_path),
        &[
            "-t",
            "-p",
            &record.root_path,
            "-c",
            target.to_string_lossy().as_ref(),
        ],
        Some(Path::new(&record.root_path)),
        15,
    )?;
    require_process_success(validation)?;
    match mode {
        NginxGlobalConfigApplyMode::Save => Ok(()),
        NginxGlobalConfigApplyMode::Reload => {
            execute_control(record, NginxControlAction::Reload).and_then(require_process_success)
        }
        NginxGlobalConfigApplyMode::Restart => {
            execute_control(record, NginxControlAction::Restart).and_then(require_process_success)
        }
    }
}

fn record_audit(
    data_directory: &Path,
    record: &NginxInstanceRecord,
    result: &NginxGlobalConfigApplyResult,
) -> NginxResult<()> {
    let path = data_directory.join("configuration-audit-v1.json");
    let mut audit = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ConfigAuditFile>(&bytes).ok())
        .unwrap_or_default();
    audit.version = 1;
    audit.events.push(ConfigAuditEvent {
        id: Uuid::new_v4().to_string(),
        instance_id: record.id.clone(),
        occurred_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        mode: result.mode,
        success: result.success,
        rolled_back: result.rolled_back,
        error_code: result.error_code.clone(),
    });
    if audit.events.len() > MAX_AUDIT_EVENTS {
        audit.events.drain(..audit.events.len() - MAX_AUDIT_EVENTS);
    }
    write_json_atomically(&path, &audit)
}

fn apply_text_patches(text: &str, mut patches: Vec<TextPatch>) -> NginxResult<String> {
    patches.sort_by_key(|patch| (std::cmp::Reverse(patch.start), std::cmp::Reverse(patch.end)));
    let mut result = text.to_owned();
    let mut last_start = text.len();
    for patch in patches {
        if patch.start > patch.end || patch.end > text.len() || patch.end > last_start {
            return Err(NginxError::new(
                "NGINX_CONFIG_PATCH_RANGE_INVALID",
                "configuration patch ranges overlap or exceed the source",
            ));
        }
        if !result.is_char_boundary(patch.start) || !result.is_char_boundary(patch.end) {
            return Err(NginxError::new(
                "NGINX_CONFIG_PATCH_RANGE_INVALID",
                "configuration patch does not align to UTF-8 boundaries",
            ));
        }
        result.replace_range(patch.start..patch.end, &patch.replacement);
        last_start = patch.start;
    }
    Ok(result)
}

fn directive_value(
    directives: &[super::dto::NginxDirective],
    name: &str,
) -> NginxResult<Option<String>> {
    singleton_directive(directives, name)
        .map(|directive| directive.map(|directive| directive.arguments.join(" ")))
}

fn singleton_directive<'a>(
    directives: &'a [super::dto::NginxDirective],
    name: &str,
) -> NginxResult<Option<&'a super::dto::NginxDirective>> {
    let mut matches = directives.iter().filter(|directive| directive.name == name);
    let first = matches.next();
    if first.is_some() && matches.next().is_some() {
        return Err(NginxError::new(
            "NGINX_CONFIG_FIELD_AMBIGUOUS",
            "an editable singleton directive appears more than once",
        ));
    }
    Ok(first)
}

fn entry_source(configuration: &NginxConfiguration) -> NginxResult<&super::dto::NginxConfigSource> {
    configuration
        .sources
        .iter()
        .find(|source| source.id == configuration.entry_source_id)
        .ok_or_else(config_not_found)
}

fn ensure_revision(graph: &NginxConfigGraph, expected_revision: &str) -> NginxResult<()> {
    if graph.revision.value != expected_revision {
        Err(NginxError::new(
            "NGINX_CONFIG_REVISION_CONFLICT",
            "the configuration changed on disk; reload before saving",
        ))
    } else {
        Ok(())
    }
}

fn require_process_success(output: super::process::ProcessOutput) -> NginxResult<()> {
    if output.success {
        Ok(())
    } else {
        Err(NginxError::new(
            "NGINX_CONFIG_RUNTIME_ACTION_FAILED",
            "nginx rejected the requested runtime action",
        ))
    }
}

fn config_not_found() -> NginxError {
    NginxError::new(
        "NGINX_CONFIG_NOT_FOUND",
        "the instance has no authorized configuration entry",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nginx_manager::dto::{
        NginxAuthorizationLevel, NginxControlBackend, NginxLifecycleState, NginxProviderIdentity,
    };

    fn record(root: &Path, config: &Path) -> NginxInstanceRecord {
        NginxInstanceRecord {
            id: "fixture".to_owned(),
            name: "Fixture".to_owned(),
            kind: "external".to_owned(),
            root_path: root.to_string_lossy().into_owned(),
            binary_path: root.join("nginx").to_string_lossy().into_owned(),
            config_path: Some(config.to_string_lossy().into_owned()),
            authorized_roots: vec![root.to_string_lossy().into_owned()],
            authorization_level: NginxAuthorizationLevel::Full,
            version: "1.28.0".to_owned(),
            configure_arguments: Vec::new(),
            binary_fingerprint: "fingerprint".to_owned(),
            provider_identity: NginxProviderIdentity {
                provider: "portable".to_owned(),
                external_id: None,
            },
            control_backend: NginxControlBackend::Portable,
            lifecycle_state: NginxLifecycleState::Available,
            created_at: "2026-08-06T00:00:00Z".to_owned(),
            updated_at: "2026-08-06T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn minimal_patches_preserve_comments_and_unknown_directives() {
        let text = "# keep\nworker_processes 1;\nunknown on;\n";
        let patched = apply_text_patches(
            text,
            vec![TextPatch {
                start: 7,
                end: 26,
                replacement: "worker_processes auto;".to_owned(),
            }],
        )
        .expect("patch");
        assert_eq!(patched, "# keep\nworker_processes auto;\nunknown on;\n");
    }

    #[test]
    fn rejects_overlapping_ranges() {
        let error = apply_text_patches(
            "worker_processes 1;",
            vec![
                TextPatch {
                    start: 0,
                    end: 10,
                    replacement: String::new(),
                },
                TextPatch {
                    start: 5,
                    end: 12,
                    replacement: String::new(),
                },
            ],
        )
        .expect_err("overlap");
        assert_eq!(error.code, "NGINX_CONFIG_PATCH_RANGE_INVALID");
    }

    #[test]
    fn validates_whitelisted_values_and_paths() {
        let patch = NginxGlobalConfigurationPatch {
            worker_processes: Some("auto".to_owned()),
            worker_rlimit_nofile: Some("0".to_owned()),
            pid: Some("../nginx.pid".to_owned()),
            error_log: Some("logs/error.log warn".to_owned()),
            top_level_includes: vec!["conf.d/*.conf".to_owned()],
            worker_connections: Some("4096".to_owned()),
            multi_accept: Some("on".to_owned()),
            accept_mutex: Some("off".to_owned()),
            accept_mutex_delay: Some("500ms".to_owned()),
        };
        let errors = validate_fields(&patch);
        assert_eq!(errors.len(), 2);
        assert!(errors
            .iter()
            .any(|error| error.field == "workerRlimitNofile"));
        assert!(errors.iter().any(|error| error.field == "pid"));
    }

    #[test]
    fn parsed_patch_preserves_unknown_directives_and_comments() {
        let directory = tempfile::tempdir().expect("tempdir");
        let config = directory.path().join("nginx.conf");
        fs::write(
            &config,
            "# keep\nworker_processes 1;\nunknown_top on;\nevents { worker_connections 1024; unknown_event on; }\n",
        )
        .expect("config");
        let configuration = load_configuration(&record(directory.path(), &config)).expect("load");
        let patch = NginxGlobalConfigurationPatch {
            worker_processes: Some("auto".to_owned()),
            worker_rlimit_nofile: None,
            pid: None,
            error_log: None,
            top_level_includes: Vec::new(),
            worker_connections: Some("2048".to_owned()),
            multi_accept: Some("on".to_owned()),
            accept_mutex: None,
            accept_mutex_delay: None,
        };
        let (bytes, errors) = build_patched_bytes(&configuration, &patch).expect("patch");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(errors.is_empty());
        assert!(text.contains("# keep"));
        assert!(text.contains("unknown_top on;"));
        assert!(text.contains("unknown_event on;"));
        assert!(text.contains("worker_processes auto;"));
        assert!(text.contains("worker_connections 2048;"));
        assert!(text.contains("multi_accept on;"));
    }

    #[test]
    fn revision_conflict_is_rejected_before_writing() {
        let directory = tempfile::tempdir().expect("tempdir");
        let config = directory.path().join("nginx.conf");
        fs::write(&config, "events {}\n").expect("config");
        let configuration = load_configuration(&record(directory.path(), &config)).expect("load");
        let graph = build_config_graph(&configuration).expect("graph");
        let error = ensure_revision(&graph, "stale").expect_err("conflict");
        assert_eq!(error.code, "NGINX_CONFIG_REVISION_CONFLICT");
    }

    #[test]
    fn runtime_failure_restores_original_bytes() {
        let directory = tempfile::tempdir().expect("tempdir");
        let config = directory.path().join("nginx.conf");
        fs::write(&config, b"original").expect("original");
        let outcome = execute_config_transaction(
            &config,
            b"candidate",
            b"original",
            || Err(NginxError::new("ACTION_FAILED", "failed")),
            || Ok(()),
        )
        .expect("transaction");
        assert!(!outcome.success);
        assert_eq!(outcome.rollback_succeeded, Some(true));
        assert_eq!(fs::read(config).expect("restored"), b"original");
    }

    #[test]
    fn rollback_failure_is_reported_as_critical() {
        let directory = tempfile::tempdir().expect("tempdir");
        let config = directory.path().join("nginx.conf");
        fs::write(&config, b"original").expect("original");
        let outcome = execute_config_transaction(
            &config,
            b"candidate",
            b"original",
            || Err(NginxError::new("ACTION_FAILED", "failed")),
            || Err(NginxError::new("ROLLBACK_FAILED", "failed")),
        )
        .expect("transaction");
        assert_eq!(outcome.rollback_succeeded, Some(false));
        assert_eq!(
            outcome.error_code.as_deref(),
            Some("NGINX_CONFIG_ROLLBACK_CRITICAL")
        );
    }
}
