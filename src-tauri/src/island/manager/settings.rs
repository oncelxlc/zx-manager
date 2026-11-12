use super::*;

impl IslandManager {
    pub fn read_config(&mut self) -> Result<IslandConfig> {
        let current = self.store.load()?;
        if self.load_error.is_some() {
            self.restart()?;
        } else if current != self.config {
            if current.active_hotkeys() != self.config.active_hotkeys() {
                if let Err(error) = self.set_hotkeys(&current) {
                    self.report(error);
                }
            }
            self.adopt(current)?;
        }
        self.config()
    }

    pub fn save(&mut self, draft: IslandConfig) -> Result<IslandConfig> {
        draft.validate()?;
        let previous = self.read_config()?;
        if draft.revision != previous.revision {
            return Err("配置已被修改，请重新载入后再保存".into());
        }
        if draft == previous {
            return Ok(previous);
        }
        let keys_changed = draft.active_hotkeys() != previous.active_hotkeys();
        let startup_changed = draft.startup.launch_at_login != previous.startup.launch_at_login;
        let mut startup_applied = false;
        let saved = (|| {
            if keys_changed {
                self.set_hotkeys(&draft)?;
            }
            if startup_changed {
                native::set_autostart(draft.startup.launch_at_login)?;
                startup_applied = true;
            }
            self.store.save(&draft, previous.revision)
        })();
        match saved {
            Ok(saved) => {
                self.adopt(saved.clone())
                    .map_err(|error| format!("配置已保存，但应用到运行窗口时出错：{error}"))?;
                Ok(saved)
            }
            Err(error) => {
                if keys_changed {
                    if let Err(rollback) = self.set_hotkeys(&previous) {
                        self.report(rollback);
                    }
                }
                if startup_applied {
                    if let Err(rollback) = native::set_autostart(previous.startup.launch_at_login) {
                        self.report(rollback);
                    }
                }
                Err(error)
            }
        }
    }

    fn set_hotkeys(&self, config: &IslandConfig) -> Result<()> {
        match &self.window {
            Some(native) => native.configure_hotkeys(config.active_hotkeys()),
            None if !config.enabled => Ok(()),
            None => Err("系统快捷键尚未就绪，请重启灵动岛".into()),
        }
    }

    fn adopt(&mut self, config: IslandConfig) -> Result<()> {
        let moved = self.config.placement != config.placement;
        if moved {
            self.session_position = None;
        }
        self.config = config;
        self.app
            .emit("island://config-changed", &self.config)
            .map_err(|e| e.to_string())?;
        if moved {
            self.position_changed()?;
        }
        self.transition(Event::ConfigUpdated)
            .inspect_err(|error| self.report(error.clone()))
    }

    pub fn set_position(
        &mut self,
        display_id: String,
        x: f64,
        y: f64,
        revision: u64,
    ) -> Result<IslandConfig> {
        let mut config = self.read_config()?;
        config.revision = revision;
        config.placement.anchor = Anchor::Custom;
        config.placement.display_id = Some(display_id);
        config.placement.offset_x = x;
        config.placement.offset_y = y;
        self.save(config)
    }

    pub fn drag(&mut self) -> Result<()> {
        if self.drag_origin.is_some()
            || !self.config.interaction.drag_enabled
            || !matches!(
                self.machine.state,
                IslandState::Compact | IslandState::Expanded | IslandState::Pinned
            )
        {
            return Err("当前位置无法拖动".into());
        }
        let window = self.window.as_ref().ok_or("灵动岛窗口不可用")?;
        let origin = match window.begin_drag() {
            Ok(origin) => origin,
            Err(error) => {
                self.refresh()?;
                return Err(error);
            }
        };
        self.drag_origin = Some((origin.x, origin.y));
        self.timer = None;
        Ok(())
    }

    pub(super) fn finish_drag(&mut self) -> Result<()> {
        let Some(origin) = self.drag_origin.take() else {
            return Ok(());
        };
        if let Err(error) = self.apply_drag_position(origin) {
            self.refresh()
                .map_err(|rollback| format!("{error}；恢复岛屿位置失败：{rollback}"))?;
            return Err(error);
        }
        self.observe_initial_pointer()
    }

    fn apply_drag_position(&mut self, origin: (i32, i32)) -> Result<()> {
        let mut config = self.effective_config();
        let window = self.window.as_ref().ok_or("灵动岛窗口不可用")?;
        let at = window.outer_rect()?;
        if (at.x, at.y) == origin {
            return self.refresh();
        }
        let (display, x, y) = displays::drag_position(&self.app, &config, self.machine.state, at)?;
        config.placement.anchor = Anchor::Custom;
        config.placement.display_id = Some(display);
        config.placement.offset_x = x;
        config.placement.offset_y = y;
        if config.placement.remember_position {
            self.save(config)?;
        } else {
            self.session_position = Some(config.placement);
            self.position_changed()?;
            self.refresh()?;
        }
        Ok(())
    }

    fn position_changed(&self) -> Result<()> {
        self.app
            .emit(
                "island://position-changed",
                self.effective_config().placement,
            )
            .map_err(|e| e.to_string())
    }
}
