use super::configuration::load_configuration;
use super::dto::{
    NginxInstanceRecord, NginxLogLine, NginxLogRotationPolicy, NginxLogRotationResult,
    NginxLogSource,
};
use super::error::{NginxError, NginxResult};
use super::registry::write_json_atomically;
use chrono::{Datelike, Utc};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

pub const MAX_PAGE_BYTES: usize = 1024 * 1024;
pub const MAX_PAGE_LINES: usize = 2_000;
pub const MAX_LINE_BYTES: usize = 64 * 1024;

pub fn default_rotation_policy() -> NginxLogRotationPolicy {
    NginxLogRotationPolicy {
        retention_months: 12,
        max_archives: 120,
        automatic_scheduling: false,
    }
}

pub fn load_rotation_policy(data_directory: &Path) -> NginxLogRotationPolicy {
    fs::read(data_directory.join("log-rotation-policy-v1.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(default_rotation_policy)
}

pub fn save_rotation_policy(
    data_directory: &Path,
    policy: &NginxLogRotationPolicy,
) -> NginxResult<()> {
    if !(1..=120).contains(&policy.retention_months)
        || !(1..=1_000).contains(&policy.max_archives)
        || policy.automatic_scheduling
    {
        return Err(NginxError::new(
            "NGINX_LOG_ROTATION_POLICY_INVALID",
            "automatic scheduling is unavailable and policy limits are bounded",
        ));
    }
    write_json_atomically(&data_directory.join("log-rotation-policy-v1.json"), policy)
}

pub fn rotate_managed_source(
    source: &ResolvedLogSource,
    policy: &NginxLogRotationPolicy,
) -> NginxResult<NginxLogRotationResult> {
    if !source.dto.managed {
        return Err(NginxError::new(
            "NGINX_LOG_ROTATION_EXTERNAL_SOURCE",
            "external nginx logs are read only",
        ));
    }
    let parent = source.path.parent().ok_or_else(|| {
        NginxError::new(
            "NGINX_LOG_ROTATION_PATH_INVALID",
            "managed log has no parent",
        )
    })?;
    let now = Utc::now();
    let archive_dir = parent.join(format!("{:04}-{:02}", now.year(), now.month()));
    fs::create_dir_all(&archive_dir)
        .map_err(|error| NginxError::io("create log archive month", error))?;
    let base = source
        .path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("nginx.log");
    let mut counter = 0u16;
    let archive = loop {
        let suffix = if counter == 0 {
            String::new()
        } else {
            format!("-{counter}")
        };
        let candidate = archive_dir.join(format!(
            "{}-{}{}.log",
            base,
            now.format("%Y%m%dT%H%M%SZ"),
            suffix
        ));
        if !candidate.exists() {
            break candidate;
        }
        counter = counter.saturating_add(1);
        if counter == u16::MAX {
            return Err(NginxError::new(
                "NGINX_LOG_ROTATION_CONFLICT",
                "archive naming space is exhausted",
            ));
        }
    };
    fs::rename(&source.path, &archive)
        .map_err(|error| NginxError::io("move managed log to archive", error))?;
    if let Err(error) = fs::File::create(&source.path) {
        let _ = fs::rename(&archive, &source.path);
        return Err(NginxError::io("recreate managed log after rotation", error));
    }
    cleanup_archives(parent, policy, base)?;
    Ok(NginxLogRotationResult {
        source_id: source.dto.id.clone(),
        rotated: true,
        archive_name: archive
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned),
        error_code: None,
    })
}

fn cleanup_archives(
    parent: &Path,
    policy: &NginxLogRotationPolicy,
    base_name: &str,
) -> NginxResult<()> {
    let now = Utc::now();
    let cutoff_month = now.year() * 12 + now.month() as i32 - policy.retention_months as i32;
    let mut archives = fs::read_dir(parent)
        .map_err(|error| NginxError::io("list log archive directories", error))?
        .flatten()
        .filter(|entry| parse_month_directory(&entry.file_name()).is_some())
        .flat_map(|entry| fs::read_dir(entry.path()).into_iter().flatten().flatten())
        .filter(|entry| {
            entry.path().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&format!("{base_name}-"))
        })
        .collect::<Vec<_>>();
    archives.sort_by_key(|entry| entry.file_name());
    let remove_count = archives.len().saturating_sub(policy.max_archives as usize);
    for (index, entry) in archives.into_iter().enumerate() {
        let archive_month = entry
            .path()
            .parent()
            .and_then(|path| path.file_name())
            .and_then(parse_month_directory)
            .unwrap_or(i32::MAX);
        if index >= remove_count && archive_month >= cutoff_month {
            continue;
        }
        fs::remove_file(entry.path())
            .map_err(|error| NginxError::io("remove expired log archive", error))?;
    }
    Ok(())
}

fn parse_month_directory(value: &std::ffi::OsStr) -> Option<i32> {
    let value = value.to_str()?;
    let (year, month) = value.split_once('-')?;
    let year = year.parse::<i32>().ok()?;
    let month = month.parse::<i32>().ok()?;
    (year >= 1970 && (1..=12).contains(&month)).then_some(year * 12 + month)
}

#[derive(Clone)]
pub struct ResolvedLogSource {
    pub dto: NginxLogSource,
    pub path: PathBuf,
}

pub fn list_sources(
    record: &NginxInstanceRecord,
    data_directory: &Path,
) -> NginxResult<Vec<ResolvedLogSource>> {
    let configuration = load_configuration(record)?;
    let mut sources = Vec::new();
    for (kind, values) in [
        ("config-access", configuration.access_logs),
        ("config-error", configuration.error_logs),
    ] {
        for value in values {
            if let Some(path) = resolve_config_log(record, &value) {
                push_unique(&mut sources, resolved_source(kind, &path, false));
            }
        }
    }
    let audit = data_directory.join("configuration-audit-v1.json");
    push_unique(
        &mut sources,
        resolved_source("zxmanager-event", &audit, true),
    );
    Ok(sources)
}

pub fn read_page(path: &Path, offset: u64) -> NginxResult<(Vec<NginxLogLine>, u64, bool)> {
    let mut file = File::open(path).map_err(|error| NginxError::io("open nginx log", error))?;
    let length = file
        .metadata()
        .map_err(|error| NginxError::io("inspect nginx log", error))?
        .len();
    let offset = offset.min(length);
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| NginxError::io("seek nginx log", error))?;
    let mut bytes = Vec::new();
    file.take(MAX_PAGE_BYTES as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| NginxError::io("read nginx log page", error))?;
    let mut lines = Vec::new();
    let mut consumed = 0usize;
    for segment in bytes
        .split_inclusive(|byte| *byte == b'\n')
        .take(MAX_PAGE_LINES)
    {
        let truncated = segment.len() > MAX_LINE_BYTES;
        let visible = &segment[..segment.len().min(MAX_LINE_BYTES)];
        lines.push(NginxLogLine {
            offset: offset + consumed as u64,
            text: String::from_utf8_lossy(visible)
                .trim_end_matches(['\r', '\n'])
                .to_owned(),
            truncated,
        });
        consumed += segment.len();
    }
    let next = offset + consumed as u64;
    Ok((lines, next, next < length))
}

pub fn file_identity(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let created = metadata
        .created()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    Some(created.to_string())
}

fn resolved_source(kind: &str, path: &Path, managed: bool) -> ResolvedLogSource {
    let id = format!(
        "{:x}",
        Sha256::digest(format!("{kind}\0{}", path.display()).as_bytes())
    );
    let availability = if path.is_file() {
        "available"
    } else {
        "missing"
    };
    ResolvedLogSource {
        dto: NginxLogSource {
            id,
            kind: kind.to_owned(),
            label: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(kind)
                .to_owned(),
            display_path: (!managed).then(|| path.to_string_lossy().into_owned()),
            availability: availability.to_owned(),
            managed,
        },
        path: path.to_path_buf(),
    }
}

fn resolve_config_log(record: &NginxInstanceRecord, value: &str) -> Option<PathBuf> {
    let path_value = value.split_whitespace().next()?;
    if matches!(path_value, "off" | "stderr" | "syslog:") || path_value.contains('$') {
        return None;
    }
    let raw = Path::new(path_value);
    if raw
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }
    let path = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        Path::new(&record.root_path).join(raw)
    };
    let canonical_parent = path.parent()?.canonicalize().ok()?;
    let authorized = record
        .authorized_roots
        .iter()
        .any(|root| canonical_parent.starts_with(root));
    let file_name = path.file_name()?;
    authorized.then(|| canonical_parent.join(file_name))
}

fn push_unique(sources: &mut Vec<ResolvedLogSource>, source: ResolvedLogSource) {
    if !sources.iter().any(|item| item.dto.id == source.dto.id) {
        sources.push(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_is_bounded_and_marks_long_lines() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("access.log");
        fs::write(
            &path,
            format!("{}\nsecond\n", "x".repeat(MAX_LINE_BYTES + 10)),
        )
        .expect("log");
        let (lines, _, _) = read_page(&path, 0).expect("page");
        assert_eq!(lines.len(), 2);
        assert!(lines[0].truncated);
        assert_eq!(lines[0].text.len(), MAX_LINE_BYTES);
    }

    #[test]
    fn external_sources_are_never_rotated() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("access.log");
        fs::write(&path, "line\n").expect("log");
        let source = resolved_source("config-access", &path, false);
        let error =
            rotate_managed_source(&source, &default_rotation_policy()).expect_err("external");
        assert_eq!(error.code, "NGINX_LOG_ROTATION_EXTERNAL_SOURCE");
        assert!(path.is_file());
    }

    #[test]
    fn managed_rotation_recreates_source_and_archives_content() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("audit.json");
        fs::write(&path, "event\n").expect("log");
        let source = resolved_source("zxmanager-event", &path, true);
        let result = rotate_managed_source(&source, &default_rotation_policy()).expect("rotate");
        assert!(result.rotated);
        assert_eq!(fs::read_to_string(&path).expect("new source"), "");
        let archive = directory
            .path()
            .join(Utc::now().format("%Y-%m").to_string())
            .join(result.archive_name.expect("archive"));
        assert_eq!(
            fs::read_to_string(archive).expect("archive content"),
            "event\n"
        );
    }
}
