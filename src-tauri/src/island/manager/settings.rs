use super::*;

impl IslandManager {
    pub fn read_config(&mut self) -> Result<IslandConfig> {
        let current = self.store.load()?;
        if self.load_error.is_some() {
            self.restart()?;
        } else if current != self.config {
            self.update_config(current, false)?;
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
        self.update_config(draft, true)
    }

    fn update_config(&mut self, next: IslandConfig, persist: bool) -> Result<IslandConfig> {
        let startup_changed =
            persist && next.startup.launch_at_login != self.config.startup.launch_at_login;
        if startup_changed {
            native::set_autostart(next.startup.launch_at_login)?;
        }
        let previous = std::mem::replace(&mut self.config, next);
        let moved = previous.placement != self.config.placement;
        let redraw = moved
            || previous.enabled != self.config.enabled
            || previous.edge != self.config.edge
            || previous.interaction != self.config.interaction
            || previous.animation != self.config.animation
            || previous.appearance != self.config.appearance
            || previous.notifications != self.config.notifications;
        let machine = self.machine.clone();
        let position = self.session_position.clone();
        let timer = self.timer;
        if moved {
            self.session_position = None;
        }
        let result = (|| {
            if redraw {
                self.transition(Event::ConfigUpdated)?;
            }
            if persist {
                self.config = self.store.save(&self.config, previous.revision)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.config = previous;
            self.machine = machine;
            self.session_position = position;
            self.timer = timer;
            if redraw {
                if let Err(rollback) = self.refresh() {
                    self.report(rollback);
                }
            }
            if startup_changed {
                if let Err(rollback) = native::set_autostart(self.config.startup.launch_at_login) {
                    self.report(rollback);
                }
            }
            return Err(error);
        }
        if moved {
            if let Err(error) = self.position_changed() {
                self.report(error);
            }
        }
        if let Err(error) = self.app.emit("island://config-changed", &self.config) {
            self.report(format!("无法发布配置更新：{error}"));
        }
        Ok(self.config.clone())
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
