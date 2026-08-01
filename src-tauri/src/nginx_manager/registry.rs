use super::dto::NginxInstanceRecord;
use super::error::{NginxError, NginxResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

const REGISTRY_VERSION: u32 = 1;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxRegistry {
    version: u32,
    pub instances: Vec<NginxInstanceRecord>,
}

impl NginxRegistry {
    pub fn load(path: &Path) -> NginxResult<Self> {
        if !path.exists() {
            return Ok(Self {
                version: REGISTRY_VERSION,
                instances: Vec::new(),
            });
        }
        let bytes = fs::read(path).map_err(|error| NginxError::io("read registry", error))?;
        match serde_json::from_slice::<Self>(&bytes) {
            Ok(registry) if registry.version == REGISTRY_VERSION => Ok(registry),
            Ok(_) => Err(NginxError::new(
                "NGINX_REGISTRY_VERSION_UNSUPPORTED",
                "the registry version is not supported",
            )),
            Err(_error) => {
                preserve_corrupt_registry(path)?;
                let registry = Self {
                    version: REGISTRY_VERSION,
                    instances: Vec::new(),
                };
                registry.save(path)?;
                Ok(registry)
            }
        }
    }

    pub fn save(&self, path: &Path) -> NginxResult<()> {
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

fn preserve_corrupt_registry(path: &Path) -> NginxResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let backup = parent.join(format!(
        "registry-v1.corrupt-{}-{}.json",
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

    #[test]
    fn corrupt_registry_is_preserved_and_reset() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("registry-v1.json");
        fs::write(&path, b"not-json").expect("fixture");

        let registry = NginxRegistry::load(&path).expect("load");

        assert!(registry.instances.is_empty());
        assert_eq!(
            fs::read_dir(directory.path())
                .expect("entries")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
                .count(),
            1
        );
        NginxRegistry::load(&path).expect("reloaded valid registry");
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
