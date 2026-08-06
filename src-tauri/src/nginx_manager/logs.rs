use super::configuration::load_configuration;
use super::dto::{NginxInstanceRecord, NginxLogLine, NginxLogSource};
use super::error::{NginxError, NginxResult};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

pub const MAX_PAGE_BYTES: usize = 1024 * 1024;
pub const MAX_PAGE_LINES: usize = 2_000;
pub const MAX_LINE_BYTES: usize = 64 * 1024;

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
}
