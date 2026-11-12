use super::geometry::{Display, Rect};
use super::state::IslandState;
use serde_json::json;
use tauri::ipc::Origin;

#[test]
fn compiled_acl_grants_island_management_only_to_main() {
    let mut context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let acl = context.runtime_authority_mut();
    for command in [
        "island_get_config",
        "island_get_runtime_status",
        "island_set_pinned",
        "island_hide",
        "plugin:event|listen",
        "plugin:event|unlisten",
        "island_update_config",
        "island_set_enabled",
        "island_show",
        "island_toggle",
        "island_restart",
        "island_preview",
        "island_reset_config",
        "island_get_displays",
        "island_get_performance_stats",
        "island_set_position",
    ] {
        for window in ["main", "island", "other"] {
            assert_eq!(
                acl.resolve_access(command, window, window, &Origin::Local)
                    .is_some(),
                window == "main",
                "{window} calling {command}"
            );
        }
    }
}

#[test]
fn ipc_uses_frontend_state_and_display_names() {
    assert_eq!(
        serde_json::to_value(IslandState::EdgePeek).unwrap(),
        "edgePeek"
    );
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
