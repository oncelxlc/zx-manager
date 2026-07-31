fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "greet",
        "complete_startup",
        "get_system_summary",
        "get_system_information",
        "get_network_monitor_capabilities",
        "get_network_monitor_status",
        "prepare_network_monitor",
        "subscribe_network_realtime",
        "unsubscribe_network_realtime",
        "query_network_usage",
        "set_network_monitor_enabled",
        "set_network_monitor_sample_interval",
        "clear_network_usage",
        "select_nginx_directory",
        "inspect_nginx_directory",
        "register_nginx_instance",
        "list_nginx_instances",
        "refresh_nginx_instance",
        "authorize_nginx_instance_root",
        "unregister_nginx_instance",
        "get_nginx_release_status",
        "check_nginx_updates",
    ]);
    let attributes = tauri_build::Attributes::new().app_manifest(manifest);

    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}
