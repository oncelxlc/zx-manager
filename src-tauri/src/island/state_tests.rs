use super::*;

#[test]
fn choosing_a_message_source_expands_once_and_respects_pin_and_hide() {
    let mut config = IslandConfig::default();
    config.interaction.click_expand = false;
    let mut machine = Machine::new(&config);
    for _ in 0..2 {
        machine.apply(Event::OpenNotifications, &config);
        assert_eq!(machine.state, IslandState::Expanded);
    }
    machine.apply(Event::Pin, &config);
    machine.apply(Event::OpenNotifications, &config);
    assert_eq!(machine.state, IslandState::Pinned);
    machine.apply(Event::Hide, &config);
    machine.apply(Event::OpenNotifications, &config);
    assert_eq!(machine.state, IslandState::Hidden);
}

#[test]
fn disabled_hidden_and_pinned_ignore_automatic_events() {
    let config = IslandConfig::default();
    let mut machine = Machine::new(&config);
    machine.apply(Event::Pin, &config);
    for event in [
        Event::PointerLeave,
        Event::CollapseTimeout,
        Event::HideTimeout,
        Event::FullscreenEntered,
        Event::OutsideClick,
    ] {
        machine.apply(event, &config);
        assert_eq!(machine.state, IslandState::Pinned);
    }
    machine.apply(Event::Hide, &config);
    machine.apply(Event::PointerEnter, &config);
    assert_eq!(machine.state, IslandState::Hidden);
    machine.apply(Event::Disable, &config);
    for event in [
        Event::Show,
        Event::Pin,
        Event::Toggle,
        Event::FullscreenExited,
    ] {
        machine.apply(event, &config);
        assert_eq!(machine.state, IslandState::Disabled);
    }
}

#[test]
fn interrupted_hover_and_peek_wake_cancel_stale_timeouts() {
    let mut config = IslandConfig::default();
    config.edge.mode = EdgeMode::Peek;
    let mut machine = Machine::new(&config);
    machine.apply(Event::PointerEnter, &config);
    assert_eq!(machine.state, IslandState::Expanded);
    assert_eq!(
        machine.apply(Event::PointerLeave, &config).timer,
        TimerAction::After {
            ms: config.interaction.collapse_delay_ms,
            event: Event::CollapseTimeout
        }
    );
    assert_eq!(
        machine.apply(Event::PointerLeave, &config).timer,
        TimerAction::Keep
    );
    assert_eq!(
        machine.apply(Event::PointerEnter, &config).timer,
        TimerAction::Cancel
    );
    machine.apply(Event::CollapseTimeout, &config);
    assert_eq!(machine.state, IslandState::Expanded);
    machine.apply(Event::PointerLeave, &config);
    assert_eq!(
        machine.apply(Event::CollapseTimeout, &config).timer,
        TimerAction::After {
            ms: config.edge.hide_delay_ms,
            event: Event::HideTimeout
        }
    );
    machine.apply(Event::HideTimeout, &config);
    assert_eq!(machine.state, IslandState::EdgePeek);
    machine.apply(Event::PointerEnter, &config);
    machine.apply(Event::PointerLeave, &config);
    machine.apply(Event::WakeTimeout, &config);
    assert_eq!(machine.state, IslandState::EdgePeek);
    machine.apply(Event::PointerEnter, &config);
    assert!(!machine.apply(Event::WakeTimeout, &config).observe_pointer);
    assert_eq!(machine.state, IslandState::Compact);
}

#[test]
fn manual_peek_wake_observes_the_stationary_pointer_and_rearms_hide() {
    let mut config = IslandConfig::default();
    config.edge.mode = EdgeMode::Peek;
    let mut machine = Machine::new(&config);
    machine.apply(Event::PointerLeave, &config);
    machine.apply(Event::HideTimeout, &config);
    for wake in [Event::Show, Event::Toggle] {
        assert!(machine.apply(wake, &config).observe_pointer);
        assert_eq!(machine.state, IslandState::Compact);
        assert_eq!(
            machine.apply(Event::PointerLeave, &config).timer,
            TimerAction::After {
                ms: config.edge.hide_delay_ms,
                event: Event::HideTimeout
            }
        );
        machine.apply(Event::HideTimeout, &config);
        assert_eq!(machine.state, IslandState::EdgePeek);
    }
}

#[test]
fn fullscreen_restores_previous_state_and_respects_explicit_user_actions() {
    let mut config = IslandConfig::default();
    let mut machine = Machine::new(&config);
    machine.apply(Event::Click, &config);
    machine.apply(Event::FullscreenEntered, &config);
    machine.apply(Event::FullscreenEntered, &config);
    assert_eq!(machine.state, IslandState::Suspended);
    machine.apply(Event::FullscreenExited, &config);
    assert_eq!(machine.state, IslandState::Expanded);
    machine.apply(Event::FullscreenEntered, &config);
    machine.apply(Event::Show, &config);
    config.appearance.opacity = 0.5;
    machine.apply(Event::ConfigUpdated, &config);
    machine.apply(Event::FullscreenEntered, &config);
    assert_eq!(machine.state, IslandState::Compact);
    machine.apply(Event::Hide, &config);
    machine.apply(Event::FullscreenExited, &config);
    assert_eq!(machine.state, IslandState::Hidden);
}

#[test]
fn disable_peek_while_suspended_changes_the_restore_target() {
    let mut config = IslandConfig::default();
    config.edge.mode = EdgeMode::Peek;
    let mut machine = Machine::new(&config);
    machine.apply(Event::PointerLeave, &config);
    machine.apply(Event::HideTimeout, &config);
    machine.apply(Event::FullscreenEntered, &config);
    config.edge.auto_hide = false;
    machine.apply(Event::ConfigUpdated, &config);
    machine.apply(Event::FullscreenExited, &config);
    assert_eq!(machine.state, IslandState::Compact);
}
