use super::dto::NginxInstanceRecord;
use super::error::{NginxError, NginxResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const REGISTRY_VERSION: u32 = 2;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRegistry {
    version: u32,
    instances: Vec<NginxInstanceRecord>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxRegistry {
    version: u32,
    pub instance: Option<NginxInstanceRecord>,
    #[serde(skip)]
    migration_candidates: Vec<NginxInstanceRecord>,
    #[serde(skip)]
    legacy_path: Option<PathBuf>,
}

impl NginxRegistry {
    pub fn load(path: &Path, legacy_path: &Path) -> NginxResult<Self> {
        if path.is_file() {
            let bytes = fs::read(path).map_err(|error| NginxError::io("read registry", error))?;
            return match serde_json::from_slice::<Self>(&bytes) {
                Ok(registry) if registry.version == REGISTRY_VERSION => Ok(registry),
                Ok(_) => Err(NginxError::new(
                    "NGINX_REGISTRY_VERSION_UNSUPPORTED",
                    "the registry version is not supported",
                )),
                Err(_) => {
                    preserve_corrupt_registry(path)?;
                    let registry = Self::empty();
                    registry.save(path)?;
                    Ok(registry)
                }
            };
        }

        if !legacy_path.is_file() {
            return Ok(Self::empty());
        }
        let bytes = fs::read(legacy_path)
            .map_err(|error| NginxError::io("read legacy nginx registry", error))?;
        let legacy = serde_json::from_slice::<LegacyRegistry>(&bytes).map_err(|_| {
            NginxError::new(
                "NGINX_REGISTRY_INVALID",
                "the legacy nginx registry could not be decoded",
            )
        })?;
        if legacy.version != 1 {
            return Err(NginxError::new(
                "NGINX_REGISTRY_VERSION_UNSUPPORTED",
                "the legacy registry version is not supported",
            ));
        }
        if legacy.instances.len() > 1 {
            return Ok(Self {
                version: REGISTRY_VERSION,
                instance: None,
                migration_candidates: legacy.instances,
                legacy_path: Some(legacy_path.to_path_buf()),
            });
        }
        let registry = Self {
            version: REGISTRY_VERSION,
            instance: legacy.instances.into_iter().next(),
            migration_candidates: Vec::new(),
            legacy_path: None,
        };
        registry.save(path)?;
        Ok(registry)
    }

    pub fn empty() -> Self {
        Self {
            version: REGISTRY_VERSION,
            instance: None,
            migration_candidates: Vec::new(),
            legacy_path: None,
        }
    }

    pub fn migration_required(&self) -> bool {
        !self.migration_candidates.is_empty()
    }

    pub fn migration_candidates(&self) -> &[NginxInstanceRecord] {
        &self.migration_candidates
    }

    pub fn resolve_migration(&mut self, path: &Path, keep_instance_id: &str) -> NginxResult<()> {
        if !self.migration_required() {
            return Err(NginxError::new(
                "NGINX_MIGRATION_NOT_REQUIRED",
                "the registry does not require migration",
            ));
        }
        let selected = self
            .migration_candidates
            .iter()
            .find(|candidate| candidate.id == keep_instance_id)
            .cloned()
            .ok_or_else(|| {
                NginxError::new(
                    "NGINX_MIGRATION_INSTANCE_INVALID",
                    "the selected migration instance was not found",
                )
            })?;
        if let Some(legacy_path) = self.legacy_path.as_deref() {
            archive_legacy_registry(legacy_path)?;
        }
        self.instance = Some(selected);
        self.migration_candidates.clear();
        self.legacy_path = None;
        self.save(path)
    }

    pub fn insert(&mut self, path: &Path, record: NginxInstanceRecord) -> NginxResult<()> {
        if self.migration_required() {
            return Err(NginxError::new(
                "NGINX_REGISTRY_MIGRATION_REQUIRED",
                "resolve the legacy multi-instance registry first",
            ));
        }
        if self.instance.is_some() {
            return Err(NginxError::new(
                "NGINX_INSTANCE_LIMIT_REACHED",
                "only one nginx instance can be registered",
            ));
        }
        self.instance = Some(record);
        self.save(path)
    }

    pub fn save(&self, path: &Path) -> NginxResult<()> {
        if self.migration_required() {
            return Err(NginxError::new(
                "NGINX_REGISTRY_MIGRATION_REQUIRED",
                "resolve the legacy multi-instance registry first",
            ));
        }
        write_json_atomically(path, self)
    }
}

pub(super) fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> NginxResult<()> {
    let parent = path.parent().ok_or_else(|| {
        NginxError::new("NGINX_REGISTRY_PATH_INVALID", "registry path has no parent")
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| NginxError::io("create registry directory", error))?;
    let temporary_path = parent.join(format!(".registry-{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| NginxError::new("NGINX_REGISTRY_SERIALIZE_FAILED", error.to_string()))?;
    let result = (|| {
        let mut temporary = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary_path)
            .map_err(|error| NginxError::io("create registry temporary file", error))?;
        temporary
            .write_all(&bytes)
            .map_err(|error| NginxError::io("write registry temporary file", error))?;
        temporary
            .sync_all()
            .map_err(|error| NginxError::io("sync registry temporary file", error))?;
        atomic_replace(&temporary_path, path)?;
        sync_directory(parent)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn archive_legacy_registry(path: &Path) -> NginxResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let archive = parent.join(format!(
        "registry-v1.migrated-{}-{}.json",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        Uuid::new_v4()
    ));
    fs::copy(path, archive)
        .map(|_| ())
        .map_err(|error| NginxError::io("archive legacy nginx registry", error))
}

fn preserve_corrupt_registry(path: &Path) -> NginxResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let backup = parent.join(format!(
        "registry-v2.corrupt-{}-{}.json",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        Uuid::new_v4()
    ));
    fs::copy(path, backup)
        .map(|_| ())
        .map_err(|error| NginxError::io("preserve corrupt registry", error))
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> NginxResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|error| NginxError::new("NGINX_REGISTRY_REPLACE_FAILED", error.to_string()))
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> NginxResult<()> {
    fs::rename(source, destination)
        .map_err(|error| NginxError::io("replace registry atomically", error))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> NginxResult<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| NginxError::io("sync registry directory", error))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> NginxResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nginx_manager::dto::{
        NginxAuthorizationLevel, NginxControlBackend, NginxLifecycleState, NginxProviderIdentity,
    };

    fn record(id: &str) -> NginxInstanceRecord {
        NginxInstanceRecord {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: "external".to_owned(),
            root_path: format!("C:/{id}"),
            binary_path: format!("C:/{id}/nginx.exe"),
            config_path: None,
            authorized_roots: vec![format!("C:/{id}")],
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
            created_at: "2026-08-01T00:00:00Z".to_owned(),
            updated_at: "2026-08-01T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn legacy_multi_instance_requires_explicit_resolution() {
        let directory = tempfile::tempdir().expect("tempdir");
        let legacy_path = directory.path().join("registry-v1.json");
        let path = directory.path().join("registry-v2.json");
        write_json_atomically(
            &legacy_path,
            &LegacyRegistry {
                version: 1,
                instances: vec![record("one"), record("two")],
            },
        )
        .expect("legacy fixture");

        let mut registry = NginxRegistry::load(&path, &legacy_path).expect("load");
        assert!(registry.migration_required());
        registry.resolve_migration(&path, "two").expect("resolve");
        assert_eq!(
            registry.instance.as_ref().map(|value| value.id.as_str()),
            Some("two")
        );
        assert!(path.is_file());
        assert_eq!(
            fs::read_dir(directory.path())
                .expect("entries")
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("registry-v1.migrated-"))
                .count(),
            1
        );
    }

    #[test]
    fn legacy_empty_and_single_registries_migrate_automatically() {
        for (count, expected) in [(0, None), (1, Some("one"))] {
            let directory = tempfile::tempdir().expect("tempdir");
            let legacy_path = directory.path().join("registry-v1.json");
            let path = directory.path().join("registry-v2.json");
            write_json_atomically(
                &legacy_path,
                &LegacyRegistry {
                    version: 1,
                    instances: if count == 0 {
                        Vec::new()
                    } else {
                        vec![record("one")]
                    },
                },
            )
            .expect("legacy fixture");
            let registry = NginxRegistry::load(&path, &legacy_path).expect("load");
            assert!(!registry.migration_required());
            assert_eq!(
                registry.instance.as_ref().map(|item| item.id.as_str()),
                expected
            );
            assert!(path.is_file());
        }
    }

    #[test]
    fn concurrent_registration_keeps_the_singleton_invariant() {
        use std::sync::{Arc, Mutex};

        let directory = tempfile::tempdir().expect("tempdir");
        let path = Arc::new(directory.path().join("registry-v2.json"));
        let registry = Arc::new(Mutex::new(NginxRegistry::empty()));
        let handles = ["one", "two"].map(|id| {
            let path = Arc::clone(&path);
            let registry = Arc::clone(&registry);
            std::thread::spawn(move || registry.lock().unwrap().insert(&path, record(id)))
        });
        let results = handles.map(|handle| handle.join().unwrap());
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter_map(|result| result.as_ref().err())
                .map(|error| error.code)
                .collect::<Vec<_>>(),
            vec!["NGINX_INSTANCE_LIMIT_REACHED"]
        );
    }

    #[test]
    fn corrupt_v2_registry_is_preserved_and_reset() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("registry-v2.json");
        fs::write(&path, b"not-json").expect("fixture");

        let registry =
            NginxRegistry::load(&path, &directory.path().join("registry-v1.json")).expect("load");

        assert!(registry.instance.is_none());
        assert_eq!(
            fs::read_dir(directory.path())
                .expect("entries")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
                .count(),
            1
        );
    }
}
