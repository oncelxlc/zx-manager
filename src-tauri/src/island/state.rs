use super::config::{EdgeMode, IslandConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IslandState {
    Disabled,
    Hidden,
    EdgePeek,
    Compact,
    Expanded,
    Pinned,
    Suspended,
}

impl IslandState {
    pub fn is_visible(self) -> bool {
        !matches!(self, Self::Disabled | Self::Hidden | Self::Suspended)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Event {
    Enable,
    Disable,
    Show,
    Hide,
    Toggle,
    PointerEnter,
    PointerLeave,
    Click,
    OpenNotifications,
    OutsideClick,
    Pin,
    Unpin,
    CollapseTimeout,
    HideTimeout,
    WakeTimeout,
    FullscreenEntered,
    FullscreenExited,
    DisplayChanged,
    ConfigUpdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerAction {
    Keep,
    Cancel,
    After { ms: u32, event: Event },
}

pub struct Effects {
    pub timer: TimerAction,
    pub observe_pointer: bool,
}

#[derive(Debug, Clone)]
pub struct Machine {
    pub state: IslandState,
    inside: Option<bool>,
    fullscreen: bool,
    fullscreen_policy: bool,
    resume: Option<IslandState>,
}

impl Machine {
    pub fn new(config: &IslandConfig) -> Self {
        let state = match (
            config.enabled,
            config.startup.show_on_start,
            config.startup.start_compact,
        ) {
            (false, _, _) => IslandState::Disabled,
            (true, false, _) => IslandState::Hidden,
            (true, true, true) => IslandState::Compact,
            (true, true, false) => IslandState::Expanded,
        };
        Self {
            state,
            inside: None,
            fullscreen: false,
            fullscreen_policy: config.edge.hide_on_fullscreen,
            resume: None,
        }
    }

    fn pointer_timer(&self, config: &IslandConfig) -> TimerAction {
        use Event::{CollapseTimeout, HideTimeout, WakeTimeout};
        use IslandState::{Compact, EdgePeek, Expanded};
        match (self.inside, self.state) {
            (Some(true), EdgePeek) => TimerAction::After {
                ms: config.edge.wake_delay_ms,
                event: WakeTimeout,
            },
            (Some(false), Expanded) if config.interaction.auto_collapse => TimerAction::After {
                ms: config.interaction.collapse_delay_ms,
                event: CollapseTimeout,
            },
            (Some(false), Compact | Expanded)
                if config.edge.auto_hide
                    && match config.edge.mode {
                        EdgeMode::Disabled => false,
                        EdgeMode::Compact => self.state == Expanded,
                        EdgeMode::Peek | EdgeMode::FullHide => true,
                    } =>
            {
                TimerAction::After {
                    ms: config.edge.hide_delay_ms,
                    event: HideTimeout,
                }
            }
            _ => TimerAction::Cancel,
        }
    }

    fn set(&mut self, next: IslandState) -> TimerAction {
        self.state = next;
        self.resume = None;
        self.inside = None;
        TimerAction::Cancel
    }

    fn suspend(&mut self, config: &IslandConfig) -> TimerAction {
        if self.fullscreen
            && config.edge.hide_on_fullscreen
            && matches!(
                self.state,
                IslandState::Compact | IslandState::EdgePeek | IslandState::Expanded
            )
        {
            self.resume = Some(self.state);
            self.state = IslandState::Suspended;
            self.inside = None;
            TimerAction::Cancel
        } else {
            TimerAction::Keep
        }
    }

    pub fn apply(&mut self, event: Event, config: &IslandConfig) -> Effects {
        use Event::*;
        use IslandState::{Compact, Disabled, EdgePeek, Expanded, Hidden, Pinned, Suspended};

        let previous = self.state;
        let timer = match event {
            Disable => self.set(Disabled),
            Enable if self.state == Disabled => self.set(Compact),
            FullscreenEntered if !self.fullscreen => {
                self.fullscreen = true;
                self.suspend(config)
            }
            FullscreenExited => {
                self.fullscreen = false;
                if self.state == Suspended {
                    self.set(self.resume.unwrap_or(Compact))
                } else {
                    TimerAction::Keep
                }
            }
            ConfigUpdated => {
                let policy_changed = self.fullscreen_policy != config.edge.hide_on_fullscreen;
                self.fullscreen_policy = config.edge.hide_on_fullscreen;
                if !config.enabled {
                    self.set(Disabled)
                } else if self.state == Disabled {
                    self.set(Compact)
                } else {
                    if self.state == Pinned && !config.interaction.pin_enabled {
                        self.state = Expanded;
                        self.suspend(config);
                    }
                    if policy_changed {
                        if self.state == Suspended && !self.fullscreen_policy {
                            self.set(self.resume.unwrap_or(Compact));
                        } else {
                            self.suspend(config);
                        }
                    }
                    if !(config.edge.auto_hide && config.edge.mode == EdgeMode::Peek) {
                        if self.state == EdgePeek {
                            self.set(Compact);
                        }
                        if self.resume == Some(EdgePeek) {
                            self.resume = Some(Compact);
                        }
                    }
                    self.pointer_timer(config)
                }
            }
            _ if self.state == Disabled => TimerAction::Keep,
            Hide => self.set(Hidden),
            Show | Toggle if matches!(self.state, Hidden | EdgePeek | Suspended) => {
                self.set(Compact)
            }
            Toggle => self.set(Hidden),
            PointerEnter | PointerLeave
                if matches!(self.state, Compact | Expanded | EdgePeek | Pinned) =>
            {
                let entering = event == PointerEnter;
                if self.inside == Some(entering) {
                    TimerAction::Keep
                } else {
                    self.inside = Some(entering);
                    if entering && self.state == Compact && config.interaction.hover_expand {
                        self.state = Expanded;
                    }
                    self.pointer_timer(config)
                }
            }
            Click if self.state == EdgePeek => self.set(Compact),
            OpenNotifications if self.state == Compact => self.set(Expanded),
            Click
                if config.interaction.click_expand && matches!(self.state, Compact | Expanded) =>
            {
                self.state = if self.state == Expanded {
                    Compact
                } else {
                    Expanded
                };
                TimerAction::Cancel
            }
            Pin if config.interaction.pin_enabled && matches!(self.state, Compact | Expanded) => {
                self.state = Pinned;
                TimerAction::Cancel
            }
            Unpin if self.state == Pinned => {
                self.state = Expanded;
                self.suspend(config);
                self.pointer_timer(config)
            }
            OutsideClick if self.state == Expanded && config.interaction.outside_collapse => {
                self.inside = Some(false);
                self.state = Compact;
                self.pointer_timer(config)
            }
            CollapseTimeout
                if self.state == Expanded
                    && self.inside == Some(false)
                    && config.interaction.auto_collapse =>
            {
                self.state = Compact;
                self.pointer_timer(config)
            }
            WakeTimeout if self.state == EdgePeek && self.inside == Some(true) => self.set(Compact),
            HideTimeout
                if matches!(self.state, Compact | Expanded)
                    && self.inside == Some(false)
                    && config.edge.auto_hide =>
            {
                match config.edge.mode {
                    EdgeMode::Compact => self.set(Compact),
                    EdgeMode::Peek => self.set(EdgePeek),
                    EdgeMode::FullHide => self.set(Hidden),
                    EdgeMode::Disabled => TimerAction::Keep,
                }
            }
            _ => TimerAction::Keep,
        };
        // A keyboard or tray wake can reveal the island under a stationary pointer.
        let observe_pointer = matches!(self.state, Compact | Expanded)
            && (matches!(previous, Hidden | Disabled | Suspended)
                || (previous == EdgePeek && matches!(event, Show | Toggle | ConfigUpdated)));
        Effects {
            timer,
            observe_pointer,
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
