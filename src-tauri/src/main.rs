// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if zx_manager_lib::try_run_network_monitor_helper() {
        return;
    }
    zx_manager_lib::run()
}
