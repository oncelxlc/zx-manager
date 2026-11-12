use super::*;
use serde::Deserialize;
use tauri::WebviewWindowBuilder;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MainSnapshot {
    route: String,
    dependencies_expanded: bool,
    #[serde(default, deserialize_with = "read_draft")]
    draft: Option<IslandConfig>,
}

fn read_draft<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> std::result::Result<Option<IslandConfig>, D::Error> {
    // A temporarily empty numeric field can serialize as null while the user is editing.
    Ok(serde_json::from_value(serde_json::Value::deserialize(input)?).ok())
}

impl Default for MainSnapshot {
    fn default() -> Self {
        Self {
            route: "#/".into(),
            dependencies_expanded: true,
            draft: None,
        }
    }
}

impl MainSnapshot {
    fn normalize(&mut self) {
        const SETTINGS: &str = "#/settings/island";
        if self.route == SETTINGS {
            return;
        }
        if let Some(group) = self.route.strip_prefix("#/settings/island?group=") {
            if matches!(
                group,
                "overview"
                    | "startup"
                    | "placement"
                    | "edge"
                    | "notifications"
                    | "appearance"
                    | "animation"
                    | "interaction"
            ) {
                return;
            }
            self.route = SETTINGS.into();
        } else {
            self.route = "#/".into();
            self.draft = None;
        }
    }
}

#[derive(Default)]
pub(super) struct MainWindow {
    snapshot: MainSnapshot,
    capture_id: u64,
    destroying: bool,
    reopen: Option<bool>,
    restoring: bool,
}

impl IslandManager {
    pub fn close_main(&mut self) -> Result<()> {
        let Some(window) = self.app.get_webview_window("main") else {
            return Ok(());
        };
        if self.main_window.destroying {
            return Ok(());
        }
        self.main_window.capture_id += 1;
        let id = self.main_window.capture_id;
        let worker = self.worker.clone();
        window.eval_with_callback(
            "window.__zxMainSession ? {route: location.hash, dependenciesExpanded: window.__zxMainSession.dependenciesExpanded, draft: window.__zxMainSession.draft} : null",
            move |json| {
                let _ = worker.post(move |manager| {
                    if id != manager.main_window.capture_id { return Ok(()); }
                    let snapshot: Option<MainSnapshot> = serde_json::from_str(&json)
                        .map_err(|_| "无法保存当前页面状态，请重试关闭主窗口".to_string())?;
                    if let Some(mut snapshot) = snapshot {
                        snapshot.normalize();
                        manager.main_window.snapshot = snapshot;
                    }
                    if let Some(window) = manager.app.get_webview_window("main") {
                        window.destroy().map_err(|e| e.to_string())?;
                        manager.main_window.destroying = true;
                    }
                    Ok(())
                });
            },
        ).map_err(|e| e.to_string())
    }

    pub fn main_destroyed(&mut self) -> Result<()> {
        self.main_window.destroying = false;
        if let Some(settings) = self.main_window.reopen.take() {
            self.open_main(settings)?;
        }
        Ok(())
    }

    pub fn restore_main(&mut self) -> Option<MainSnapshot> {
        std::mem::take(&mut self.main_window.restoring).then(|| self.main_window.snapshot.clone())
    }

    pub fn open_main(&mut self, settings: bool) -> Result<()> {
        let state = &mut self.main_window;
        state.capture_id += 1;
        if state.destroying {
            state.reopen = Some(settings || state.reopen == Some(true));
            return Ok(());
        }
        let window = if let Some(window) = self.app.get_webview_window("main") {
            if settings {
                state.snapshot.route = "#/settings/island".into();
                window
                    .eval("location.hash='/settings/island'")
                    .map_err(|e| e.to_string())?;
            }
            window
        } else {
            if settings {
                state.snapshot.route = "#/settings/island".into();
            }
            state.restoring = true;
            let config = self
                .app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .ok_or("找不到主窗口配置")?;
            WebviewWindowBuilder::from_config(&self.app, config)
                .and_then(|builder| builder.build())
                .map_err(|e| e.to_string())?
        };
        window
            .unminimize()
            .and_then(|_| window.show())
            .and_then(|_| window.set_focus())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_snapshot_keeps_only_known_pages_and_settings_drafts() {
        let mut snapshot = MainSnapshot {
            route: "#/settings/island?group=placement".into(),
            dependencies_expanded: false,
            draft: Some(IslandConfig::default()),
        };
        snapshot.normalize();
        assert_eq!(snapshot.route, "#/settings/island?group=placement");
        assert!(snapshot.draft.is_some());
        snapshot.route = "#/settings/island?group=notifications&extra=private".into();
        snapshot.normalize();
        assert_eq!(snapshot.route, "#/settings/island");
        snapshot.route = "#/unknown?private".into();
        snapshot.normalize();
        assert_eq!(snapshot.route, "#/");
        assert!(snapshot.draft.is_none());
    }

    #[test]
    fn incomplete_draft_does_not_prevent_route_recovery() {
        let data = r##"{"route":"#/settings/island?group=placement","dependenciesExpanded":false,"draft":{"placement":{"offsetX":null}}}"##;
        let snapshot: MainSnapshot = serde_json::from_str(data).unwrap();
        assert_eq!(snapshot.route, "#/settings/island?group=placement");
        assert!(!snapshot.dependencies_expanded);
        assert!(snapshot.draft.is_none());
    }
}
