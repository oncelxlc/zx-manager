use super::dto::{NginxInstanceRecord, NginxRuntimeStatus};
use super::error::{NginxError, NginxResult};
use super::manager::reject_reparse_points;
use super::registry::write_json_atomically;
use pgp::composed::{Deserializable, DetachedSignature, SignedPublicKey};
use pgp::types::KeyDetails;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
use zip::ZipArchive;

const MAX_ZIP_ENTRIES: usize = 4_096;
const MAX_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024;
const MAX_BACKUP_FILES: usize = 10_000;
const MAX_BACKUP_BYTES: u64 = 512 * 1024 * 1024;

struct DirectoryGuard(Option<PathBuf>);

impl Drop for DirectoryGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

const PINNED_KEYS: [(&str, &str, &[&str], &[u8]); 5] = [
    (
        "arut.key",
        "43387825DDB1BB97EC36BA5D007C8D7C15D87369",
        &["0AB1A7CA1283FF69D0E5F483E79FE70A6F23EB22"],
        include_bytes!("keys/arut.key"),
    ),
    (
        "pluknet.key",
        "D6786CE303D9A9022998DC6CC8464D549AF75C0A",
        &["7AB27B82D2C1EEA3402128EFBFC914095852E0BE"],
        include_bytes!("keys/pluknet.key"),
    ),
    (
        "sb.key",
        "7338973069ED3F443F4D37DFA64FD5B17ADB39A8",
        &["207882C1C3FBEFC96DCD822D02C1EAAF4ADC0B9A"],
        include_bytes!("keys/sb.key"),
    ),
    (
        "thresh.key",
        "13C82A63B603576156E30A4EA0EA981B66B0D967",
        &["D7CBD7C1A25E5D31EB5D28BCEC67AEFF1281B785"],
        include_bytes!("keys/thresh.key"),
    ),
    (
        "nginx_signing.key",
        "8540A6F18833A80E9C1653A42FD21310B49F6B46",
        &[],
        include_bytes!("keys/nginx_signing.key"),
    ),
];

pub struct PreparedUpgrade {
    pub staging_directory: PathBuf,
    pub candidate_binary: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    relative_path: String,
    sha256: String,
    size: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupMetadata {
    version: u8,
    backup_id: String,
    created_at: String,
    original_instance: NginxInstanceRecord,
    original_runtime_status: NginxRuntimeStatus,
    files: Vec<BackupFile>,
}

pub fn verify_signature(archive: &[u8], signature_bytes: &[u8]) -> NginxResult<()> {
    let (signature, _) = DetachedSignature::from_armor_single(signature_bytes).map_err(|_| {
        NginxError::new(
            "NGINX_UPGRADE_SIGNATURE_INVALID",
            "detached signature could not be parsed",
        )
    })?;
    for (_name, expected_primary, expected_subkeys, bytes) in PINNED_KEYS {
        let (parsed, _) = SignedPublicKey::from_armor_many(bytes).map_err(|_| {
            NginxError::new(
                "NGINX_UPGRADE_KEY_REJECTED",
                "a pinned nginx public key could not be parsed",
            )
        })?;
        for key in parsed {
            let key = key.map_err(|_| {
                NginxError::new(
                    "NGINX_UPGRADE_KEY_REJECTED",
                    "a pinned nginx public key was malformed",
                )
            })?;
            if format!("{:X}", key.fingerprint()) != expected_primary {
                return Err(NginxError::new(
                    "NGINX_UPGRADE_KEY_REJECTED",
                    "the nginx public key fingerprint did not match the pinned value",
                ));
            }
            // The full primary fingerprint is the trust anchor. User-ID certifications and
            // unrelated historical subkeys are not part of release-signature authorization.
            if signature.verify(&key, archive).is_ok() {
                return Ok(());
            }
            for subkey in &key.public_subkeys {
                let fingerprint = format!("{:X}", subkey.fingerprint());
                if expected_subkeys.contains(&fingerprint.as_str())
                    && signature.verify(subkey, archive).is_ok()
                {
                    subkey.verify_bindings(&key.primary_key).map_err(|_| {
                        NginxError::new(
                            "NGINX_UPGRADE_KEY_REJECTED",
                            "the nginx signing subkey binding was invalid",
                        )
                    })?;
                    return Ok(());
                }
            }
        }
    }
    Err(NginxError::new(
        "NGINX_UPGRADE_SIGNATURE_REJECTED",
        "the release signature was not made by a pinned nginx signing key",
    ))
}

pub fn extract_release(
    archive_bytes: &[u8],
    staging_parent: &Path,
    target_version: &str,
) -> NginxResult<PreparedUpgrade> {
    fs::create_dir_all(staging_parent)
        .map_err(|error| NginxError::io("create nginx staging directory", error))?;
    reject_reparse_points(staging_parent)?;
    let staging_directory = staging_parent.join(Uuid::new_v4().to_string());
    fs::create_dir(&staging_directory)
        .map_err(|error| NginxError::io("create nginx upgrade staging directory", error))?;
    let mut cleanup = DirectoryGuard(Some(staging_directory.clone()));
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes)).map_err(|_| {
        NginxError::new(
            "NGINX_UPGRADE_ARCHIVE_INVALID",
            "release ZIP could not be read",
        )
    })?;
    if archive.is_empty() || archive.len() > MAX_ZIP_ENTRIES {
        return Err(NginxError::new(
            "NGINX_UPGRADE_ARCHIVE_LIMIT_EXCEEDED",
            "release ZIP contains an invalid number of entries",
        ));
    }
    let total_size = (0..archive.len()).try_fold(0_u64, |total, index| {
        let file = archive.by_index(index).map_err(|_| {
            NginxError::new(
                "NGINX_UPGRADE_ARCHIVE_INVALID",
                "release ZIP entry is invalid",
            )
        })?;
        total.checked_add(file.size()).ok_or_else(|| {
            NginxError::new(
                "NGINX_UPGRADE_ARCHIVE_LIMIT_EXCEEDED",
                "release ZIP expanded size overflowed",
            )
        })
    })?;
    if total_size > MAX_EXTRACTED_BYTES {
        return Err(NginxError::new(
            "NGINX_UPGRADE_ARCHIVE_LIMIT_EXCEEDED",
            "release ZIP expanded size exceeds the fixed limit",
        ));
    }
    let mut roots = HashSet::new();
    let mut seen = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| {
            NginxError::new(
                "NGINX_UPGRADE_ARCHIVE_INVALID",
                "release ZIP entry is invalid",
            )
        })?;
        let enclosed = entry.enclosed_name().ok_or_else(|| {
            NginxError::new(
                "NGINX_UPGRADE_ARCHIVE_PATH_REJECTED",
                "release ZIP contains an unsafe path",
            )
        })?;
        if enclosed.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(NginxError::new(
                "NGINX_UPGRADE_ARCHIVE_PATH_REJECTED",
                "release ZIP contains an unsafe path",
            ));
        }
        let normalized = enclosed
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase();
        if !seen.insert(normalized) {
            return Err(NginxError::new(
                "NGINX_UPGRADE_ARCHIVE_DUPLICATE_PATH",
                "release ZIP contains duplicate normalized paths",
            ));
        }
        let Some(root) = enclosed.components().next() else {
            continue;
        };
        roots.insert(root.as_os_str().to_owned());
        if entry
            .unix_mode()
            .is_some_and(|mode| !matches!(mode & 0o170000, 0 | 0o040000 | 0o100000))
        {
            return Err(NginxError::new(
                "NGINX_UPGRADE_ARCHIVE_LINK_REJECTED",
                "release ZIP contains a link or special file",
            ));
        }
        let destination = staging_directory.join(&enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|error| NginxError::io("create extracted directory", error))?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| NginxError::io("create extracted parent", error))?;
            }
            let mut output = fs::File::create(&destination)
                .map_err(|error| NginxError::io("create extracted file", error))?;
            std::io::copy(&mut entry.take(MAX_EXTRACTED_BYTES + 1), &mut output)
                .map_err(|error| NginxError::io("extract release file", error))?;
            output
                .flush()
                .map_err(|error| NginxError::io("flush extracted file", error))?;
        }
        reject_reparse_points(&destination)?;
    }
    if roots.len() != 1 {
        return Err(NginxError::new(
            "NGINX_UPGRADE_ARCHIVE_ROOT_INVALID",
            "release ZIP must contain exactly one package root",
        ));
    }
    let candidate_root = staging_directory.join(roots.into_iter().next().unwrap());
    let expected_root = format!("nginx-{target_version}");
    if candidate_root
        .file_name()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case(&expected_root))
    {
        return Err(NginxError::new(
            "NGINX_UPGRADE_ARCHIVE_VERSION_MISMATCH",
            "release ZIP root does not match the target version",
        ));
    }
    let candidate_binary = candidate_root.join("nginx.exe");
    if !candidate_binary.is_file() {
        return Err(NginxError::new(
            "NGINX_UPGRADE_BINARY_MISSING",
            "release ZIP does not contain nginx.exe",
        ));
    }
    cleanup.0 = None;
    Ok(PreparedUpgrade {
        staging_directory,
        candidate_binary,
    })
}

pub fn create_backup(
    backup_parent: &Path,
    record: &NginxInstanceRecord,
    runtime_status: NginxRuntimeStatus,
) -> NginxResult<String> {
    fs::create_dir_all(backup_parent)
        .map_err(|error| NginxError::io("create nginx backup directory", error))?;
    reject_reparse_points(backup_parent)?;
    let backup_id = format!(
        "{}-{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S%.9f"),
        Uuid::new_v4()
    );
    let destination = backup_parent.join(&backup_id);
    fs::create_dir(&destination)
        .map_err(|error| NginxError::io("create nginx backup snapshot", error))?;
    let mut cleanup = DirectoryGuard(Some(destination.clone()));
    let mut files = Vec::new();
    copy_backup_file(
        Path::new(&record.binary_path),
        &destination.join("nginx.exe"),
        "nginx.exe",
        &mut files,
    )?;
    for directory in ["conf", "modules"] {
        let source = Path::new(&record.root_path).join(directory);
        if source.exists() {
            copy_backup_tree(&source, &destination.join(directory), directory, &mut files)?;
        }
    }
    if files.len() > MAX_BACKUP_FILES
        || files.iter().map(|file| file.size).sum::<u64>() > MAX_BACKUP_BYTES
    {
        return Err(NginxError::new(
            "NGINX_UPGRADE_BACKUP_LIMIT_EXCEEDED",
            "nginx critical snapshot exceeded the fixed limit",
        ));
    }
    write_json_atomically(
        &destination.join("metadata.json"),
        &BackupMetadata {
            version: 1,
            backup_id: backup_id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            original_instance: record.clone(),
            original_runtime_status: runtime_status,
            files,
        },
    )?;
    cleanup.0 = None;
    Ok(backup_id)
}

fn copy_backup_tree(
    source: &Path,
    destination: &Path,
    relative: &str,
    files: &mut Vec<BackupFile>,
) -> NginxResult<()> {
    reject_reparse_points(source)?;
    fs::create_dir_all(destination)
        .map_err(|error| NginxError::io("create backup directory", error))?;
    for entry in
        fs::read_dir(source).map_err(|error| NginxError::io("read backup source", error))?
    {
        let entry = entry.map_err(|error| NginxError::io("read backup entry", error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| NginxError::io("inspect backup entry", error))?;
        if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
            return Err(NginxError::new(
                "NGINX_UPGRADE_BACKUP_LINK_REJECTED",
                "critical snapshot contains a link or special file",
            ));
        }
        let name = entry.file_name();
        let child_relative = format!("{relative}/{}", name.to_string_lossy());
        let child_destination = destination.join(&name);
        if file_type.is_dir() {
            copy_backup_tree(&entry.path(), &child_destination, &child_relative, files)?;
        } else {
            copy_backup_file(&entry.path(), &child_destination, &child_relative, files)?;
        }
        if files.len() > MAX_BACKUP_FILES {
            return Err(NginxError::new(
                "NGINX_UPGRADE_BACKUP_LIMIT_EXCEEDED",
                "critical snapshot contains too many files",
            ));
        }
    }
    Ok(())
}

fn copy_backup_file(
    source: &Path,
    destination: &Path,
    relative_path: &str,
    files: &mut Vec<BackupFile>,
) -> NginxResult<()> {
    reject_reparse_points(source)?;
    let bytes = fs::read(source).map_err(|error| NginxError::io("read backup file", error))?;
    fs::write(destination, &bytes).map_err(|error| NginxError::io("write backup file", error))?;
    files.push(BackupFile {
        relative_path: relative_path.to_owned(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        size: bytes.len() as u64,
    });
    Ok(())
}

pub fn backup_record(backup_parent: &Path, backup_id: &str) -> NginxResult<NginxInstanceRecord> {
    let metadata = load_backup_metadata(backup_parent, backup_id)?;
    Ok(metadata.original_instance)
}

pub fn restore_binary(
    backup_parent: &Path,
    backup_id: &str,
    binary_path: &Path,
) -> NginxResult<()> {
    let metadata = load_backup_metadata(backup_parent, backup_id)?;
    let backup = backup_parent.join(backup_id).join("nginx.exe");
    let bytes = fs::read(&backup).map_err(|error| NginxError::io("read backup binary", error))?;
    let expected = metadata
        .files
        .iter()
        .find(|file| file.relative_path == "nginx.exe")
        .ok_or_else(|| {
            NginxError::new(
                "NGINX_UPGRADE_BACKUP_INVALID",
                "backup manifest does not contain nginx.exe",
            )
        })?;
    if expected.size != bytes.len() as u64
        || expected.sha256 != format!("{:x}", Sha256::digest(&bytes))
    {
        return Err(NginxError::new(
            "NGINX_UPGRADE_BACKUP_INVALID",
            "backup nginx.exe failed integrity verification",
        ));
    }
    replace_binary(&backup, binary_path)
}

fn load_backup_metadata(backup_parent: &Path, backup_id: &str) -> NginxResult<BackupMetadata> {
    if backup_id.contains(['/', '\\']) || backup_id.contains("..") {
        return Err(NginxError::new(
            "NGINX_UPGRADE_BACKUP_INVALID",
            "backup identifier is invalid",
        ));
    }
    let bytes = fs::read(backup_parent.join(backup_id).join("metadata.json"))
        .map_err(|error| NginxError::io("read backup metadata", error))?;
    serde_json::from_slice(&bytes).map_err(|_| {
        NginxError::new(
            "NGINX_UPGRADE_BACKUP_INVALID",
            "backup metadata could not be verified",
        )
    })
}

pub fn replace_binary(source: &Path, destination: &Path) -> NginxResult<()> {
    reject_reparse_points(source)?;
    reject_reparse_points(destination)?;
    let parent = destination.parent().ok_or_else(|| {
        NginxError::new("NGINX_UPGRADE_REPLACE_FAILED", "nginx binary has no parent")
    })?;
    let token = Uuid::new_v4();
    let staged = parent.join(format!(".zx-nginx-{token}.new"));
    let previous = parent.join(format!(".zx-nginx-{token}.old"));
    fs::copy(source, &staged)
        .map_err(|error| NginxError::io("stage replacement nginx binary", error))?;
    fs::rename(destination, &previous)
        .map_err(|error| NginxError::io("move current nginx binary", error))?;
    if let Err(error) = fs::rename(&staged, destination) {
        let _ = fs::rename(&previous, destination);
        return Err(NginxError::io("activate replacement nginx binary", error));
    }
    let _ = fs::remove_file(previous);
    Ok(())
}

pub fn cleanup_backups(
    backup_parent: &Path,
    retention: u8,
    preserve: Option<&str>,
) -> NginxResult<()> {
    let mut directories = fs::read_dir(backup_parent)
        .map_err(|error| NginxError::io("read nginx backup directory", error))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .collect::<Vec<_>>();
    directories.sort_by_key(|entry| entry.file_name());
    let remove_count = directories.len().saturating_sub(usize::from(retention));
    for entry in directories
        .into_iter()
        .filter(|entry| entry.file_name().to_string_lossy() != preserve.unwrap_or_default())
        .take(remove_count)
    {
        reject_reparse_points(&entry.path())?;
        fs::remove_dir_all(entry.path())
            .map_err(|error| NginxError::io("remove expired nginx backup", error))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nginx_manager::dto::{
        NginxAuthorizationLevel, NginxControlBackend, NginxLifecycleState, NginxProviderIdentity,
    };
    use std::io::Write;

    fn record(root: &Path) -> NginxInstanceRecord {
        NginxInstanceRecord {
            id: "nginx".to_owned(),
            name: "Nginx".to_owned(),
            kind: "external".to_owned(),
            root_path: root.to_string_lossy().into_owned(),
            binary_path: root.join("nginx.exe").to_string_lossy().into_owned(),
            config_path: Some(root.join("conf/nginx.conf").to_string_lossy().into_owned()),
            authorized_roots: vec![root.to_string_lossy().into_owned()],
            authorization_level: NginxAuthorizationLevel::Full,
            version: "1.28.0".to_owned(),
            configure_arguments: Vec::new(),
            binary_fingerprint: "hash".to_owned(),
            provider_identity: NginxProviderIdentity {
                provider: "portable".to_owned(),
                external_id: None,
            },
            control_backend: NginxControlBackend::Portable,
            lifecycle_state: NginxLifecycleState::Available,
            created_at: "2026-08-01T00:00:00Z".to_owned(),
            updated_at: "2026-08-01T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn embedded_nginx_keys_match_reviewed_fingerprints() {
        for (_name, expected, _subkeys, bytes) in PINNED_KEYS {
            let (mut keys, _) = SignedPublicKey::from_armor_many(bytes).unwrap();
            let key = keys.next().unwrap().unwrap();
            assert_eq!(format!("{:X}", key.fingerprint()), expected);
        }
    }

    #[test]
    #[ignore = "requires live access to nginx.org"]
    fn live_official_windows_release_signature_is_accepted() {
        use crate::nginx_manager::dto::NginxReleaseChannel;
        use crate::nginx_manager::release::ReleaseUpdateService;

        if let (Ok(archive_path), Ok(signature_path)) = (
            std::env::var("NGINX_LIVE_ARCHIVE"),
            std::env::var("NGINX_LIVE_SIGNATURE"),
        ) {
            let archive = fs::read(archive_path).unwrap();
            let signature = fs::read(signature_path).unwrap();
            verify_signature(&archive, &signature).unwrap();
            return;
        }

        let directory = tempfile::tempdir().unwrap();
        let service =
            ReleaseUpdateService::new(directory.path().join("release-cache.json")).unwrap();
        tauri::async_runtime::block_on(async {
            let (cached, _) = service
                .check(NginxReleaseChannel::Stable, true, 24)
                .await
                .unwrap();
            let (archive, signature) = service.download_release(&cached.release).await.unwrap();
            verify_signature(&archive, &signature).unwrap();
        });
    }

    #[test]
    fn rejects_zip_path_traversal() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut bytes);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("../nginx.exe", options).unwrap();
            writer.write_all(b"bad").unwrap();
            writer.finish().unwrap();
        }
        let directory = tempfile::tempdir().unwrap();
        assert!(extract_release(bytes.get_ref(), directory.path(), "1.28.0").is_err());
    }

    #[test]
    fn backup_contains_only_critical_files_and_preserves_rollback_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("nginx");
        let backups = directory.path().join("backups");
        for path in ["conf", "modules", "logs", "html"] {
            fs::create_dir_all(root.join(path)).unwrap();
        }
        fs::write(root.join("nginx.exe"), b"binary").unwrap();
        fs::write(root.join("conf/nginx.conf"), b"events {}").unwrap();
        fs::write(root.join("modules/module.dll"), b"module").unwrap();
        fs::write(root.join("logs/access.log"), b"private").unwrap();
        fs::write(root.join("html/index.html"), b"site").unwrap();
        let record = record(&root);
        let first = create_backup(&backups, &record, NginxRuntimeStatus::Stopped).unwrap();
        let second = create_backup(&backups, &record, NginxRuntimeStatus::Stopped).unwrap();
        assert!(backups.join(&first).join("nginx.exe").is_file());
        assert!(backups.join(&first).join("conf/nginx.conf").is_file());
        assert!(backups.join(&first).join("modules/module.dll").is_file());
        assert!(!backups.join(&first).join("logs").exists());
        assert!(!backups.join(&first).join("html").exists());
        cleanup_backups(&backups, 1, Some(&first)).unwrap();
        assert!(backups.join(first).is_dir());
        assert!(!backups.join(second).exists());
    }
}
