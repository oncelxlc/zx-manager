fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "greet",
        "get_system_summary",
        "get_system_information",
    ]);
    let attributes = tauri_build::Attributes::new().app_manifest(manifest);

    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}
