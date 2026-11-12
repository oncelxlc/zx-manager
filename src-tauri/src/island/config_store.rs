use super::config::{IslandConfig, SCHEMA_VERSION};
use super::geometry::Display;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

pub struct ConfigStore {
    directory: PathBuf,
    displays: Box<dyn Fn() -> Result<Vec<Display>, String> + Send>,
}

impl ConfigStore {
    pub fn new(
        config_dir: PathBuf,
        displays: impl Fn() -> Result<Vec<Display>, String> + Send + 'static,
    ) -> Self {
        Self {
            directory: config_dir,
            displays: Box::new(displays),
        }
    }

    fn lock(&self) -> Result<File, String> {
        fs::create_dir_all(&self.directory).map_err(io_error)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join("island.lock"))
            .map_err(io_error)?;
        file.lock().map_err(io_error)?;
        Ok(file)
    }

    pub fn load(&self) -> Result<IslandConfig, String> {
        let _lock = self.lock()?;
        let (config, migrated) = self.read()?;
        if migrated {
            self.write(&config)?;
        }
        Ok(config)
    }

    fn read(&self) -> Result<(IslandConfig, bool), String> {
        let bytes = match fs::read(self.directory.join("island.json")) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                if self
                    .directory
                    .join("island.json.bak")
                    .try_exists()
                    .map_err(io_error)?
                {
                    return Err("找不到 island.json，请从 island.json.bak 恢复配置".into());
                }
                return Ok((IslandConfig::default(), false));
            }
            Err(err) => return Err(io_error(err)),
        };
        let mut value: Value =
            serde_json::from_slice(&bytes).map_err(|err| format!("island.json 无法解析：{err}"))?;
        let fields = value
            .as_object_mut()
            .ok_or("island.json 必须是 JSON 对象")?;
        let version = match fields.get("schemaVersion") {
            Some(version) => version.as_u64().ok_or("配置版本必须是非负整数")?,
            None => 0,
        };
        match version {
            0..=2 => {
                fields.remove("hotkeys");
                if let Some(startup) = fields.get_mut("startup").and_then(Value::as_object_mut) {
                    startup.remove("closeToTray");
                }
                if let Some(input) = fields.get_mut("interaction").and_then(Value::as_object_mut) {
                    for key in ["outsideCollapse", "autoCollapse", "collapseDelayMs"] {
                        input.remove(key);
                    }
                }
                if let Some(notices) = fields
                    .get_mut("notifications")
                    .and_then(Value::as_object_mut)
                {
                    // A single switch cannot preserve a selective opt-out. Require an explicit opt-in.
                    for key in ["wechatEnabled", "qqEnabled", "windowsNotificationsEnabled"] {
                        if notices.remove(key) == Some(Value::Bool(false)) {
                            notices.insert("enabled".into(), false.into());
                        }
                    }
                    for key in [
                        "showPreview",
                        "previewTimeoutSeconds",
                        "accountListEnabled",
                        "animateNewMessage",
                    ] {
                        notices.remove(key);
                    }
                }
            }
            3..=5 => {}
            version if version == u64::from(SCHEMA_VERSION) => {}
            version => return Err(format!("不支持的配置版本 {version}，请升级应用")),
        }
        let migrated = version < u64::from(SCHEMA_VERSION);
        let legacy_size = |key: &str| fields.get("appearance")?.get(key)?.as_f64();
        let old_width = legacy_size("compactWidth").unwrap_or(120.0);
        let old_height = legacy_size("compactHeight").unwrap_or(30.0);
        if migrated {
            if let Some(edge) = fields.get_mut("edge").and_then(Value::as_object_mut) {
                edge.remove("triggerWidth");
                edge.remove("peekSize");
            }
            if let Some(a) = fields.get_mut("appearance").and_then(Value::as_object_mut) {
                for key in [
                    "compactWidth",
                    "compactHeight",
                    "expandedWidth",
                    "expandedHeight",
                    "radius",
                ] {
                    a.remove(key);
                }
            }
            if let Some(p) = fields.get_mut("placement").and_then(Value::as_object_mut) {
                p.remove("orientation");
                p.remove("expandDirection");
            }
        }
        fields.insert("schemaVersion".into(), SCHEMA_VERSION.into());
        let mut config: IslandConfig =
            serde_json::from_value(value).map_err(|err| format!("island.json 配置无效：{err}"))?;
        if migrated
            && (config.placement.offset_x != 0.0
                || config.placement.offset_y != 0.0
                || !matches!(
                    config.placement.anchor,
                    super::config::Anchor::TopCenter
                        | super::config::Anchor::CenterLeft
                        | super::config::Anchor::CenterRight
                ))
        {
            let displays = (self.displays)()?;
            let screen =
                super::displays::target_display(&displays, config.placement.display_id.as_deref())?;
            screen.migrate_placement(&mut config, old_width, old_height)?;
        }
        if version < 4 {
            let a = &mut config.appearance;
            let defaults = super::config::AppearanceConfig::default();
            if a.background.eq_ignore_ascii_case("#101214")
                && a.border.eq_ignore_ascii_case("#34383c")
                && a.opacity == 0.96
            {
                a.background = defaults.background;
                a.border = defaults.border;
                a.opacity = defaults.opacity;
            }
        }
        config.validate()?;
        Ok((config, migrated))
    }

    pub fn save(
        &self,
        draft: &IslandConfig,
        expected_revision: u64,
    ) -> Result<IslandConfig, String> {
        draft.validate()?;
        let _lock = self.lock()?;
        let (previous, _) = self.read()?;
        if previous.revision != expected_revision {
            return Err("配置已被修改，请重新载入后再保存".into());
        }
        if draft == &previous {
            return Ok(previous);
        }
        // JavaScript clients must be able to round-trip the revision exactly.
        if expected_revision >= (1 << 53) - 1 {
            return Err("配置版本计数已达上限".into());
        }
        let mut saved = draft.clone();
        saved.revision = expected_revision + 1;
        self.write(&saved)?;
        Ok(saved)
    }

    /// Caller holds the sidecar file lock, including across both replacements.
    fn write(&self, config: &IslandConfig) -> Result<(), String> {
        let path = self.directory.join("island.json");
        let temp = self.directory.join("island.json.tmp");
        let mut bytes = serde_json::to_vec_pretty(config).map_err(|err| err.to_string())?;
        bytes.push(b'\n');
        write_synced(&temp, &bytes).map_err(io_error)?;
        if path.try_exists().map_err(io_error)? {
            let backup = self.directory.join("island.json.bak");
            let backup_temp = self.directory.join("island.json.bak.tmp");
            let old = fs::read(&path).map_err(io_error)?;
            write_synced(&backup_temp, &old).map_err(io_error)?;
            fs::rename(&backup_temp, &backup).map_err(io_error)?;
        }
        fs::rename(&temp, &path).map_err(io_error)?;
        #[cfg(unix)]
        File::open(&self.directory)
            .and_then(|dir| dir.sync_all())
            .map_err(io_error)?;
        Ok(())
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()
}

fn io_error(error: std::io::Error) -> String {
    format!("读写灵动岛配置失败：{error}")
}

#[cfg(test)]
#[path = "config_store_tests.rs"]
mod tests;
