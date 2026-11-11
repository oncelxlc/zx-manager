use super::geometry::{Display, Rect};
use super::state::{Event, IslandState};
use serde_json::json;
use tauri::ipc::Origin;

#[test]
fn compiled_acl_grants_each_windows_view_its_commands() {
    let mut context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let acl = context.runtime_authority_mut();
    let both: &[&str] = &["main", "island"];
    let main: &[&str] = &["main"];
    let island: &[&str] = &["island"];
    for (command, allowed) in [
        ("island_get_config", both),
        ("island_get_runtime_status", both),
        ("island_set_pinned", both),
        ("island_hide", both),
        ("plugin:event|listen", both),
        ("plugin:event|unlisten", both),
        ("island_update_config", main),
        ("island_set_enabled", main),
        ("island_show", main),
        ("island_toggle", main),
        ("island_restart", main),
        ("island_preview", main),
        ("island_reset_config", main),
        ("island_get_displays", main),
        ("island_set_position", main),
        ("island_interact", island),
        ("island_drag", island),
        ("island_ready", island),
    ] {
        for window in ["main", "island", "other"] {
            assert_eq!(
                acl.resolve_access(command, window, window, &Origin::Local)
                    .is_some(),
                allowed.contains(&window),
                "{window} calling {command}"
            );
        }
    }
}

#[test]
fn ipc_uses_frontend_state_event_and_display_names() {
    assert_eq!(
        serde_json::to_value(IslandState::EdgePeek).unwrap(),
        "edgePeek"
    );
    for (wire, event) in [
        ("pointerEnter", Event::PointerEnter),
        ("pointerLeave", Event::PointerLeave),
        ("click", Event::Click),
    ] {
        assert_eq!(serde_json::from_value::<Event>(json!(wire)).unwrap(), event);
    }
    let display = Display {
        id: "left".into(),
        name: "左屏".into(),
        primary: false,
        scale_factor: 1.5,
        work: Rect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        },
    };
    assert_eq!(
        serde_json::to_value(display).unwrap(),
        json!({
            "id": "left", "name": "左屏", "primary": false, "scaleFactor": 1.5
        })
    );
}
