use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "zx-once-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn migration_keeps_old_bytes_and_supplies_missing_defaults() {
    let dir = Directory::new();
    let store = ConfigStore::new(dir.0.clone());
    let old = br#"{"revision":4,"startup":{"showOnStart":false}}"#;
    fs::write(dir.0.join("island.json"), old).unwrap();
    let config = store.load().unwrap();
    assert_eq!(config.schema_version, SCHEMA_VERSION);
    assert!(!config.startup.show_on_start);
    assert_eq!(config.animation, IslandConfig::default().animation);
    assert_eq!(fs::read(dir.0.join("island.json.bak")).unwrap(), old);
    assert_eq!(store.save(&config, 4).unwrap().revision, 4);
    let changed = IslandConfig {
        enabled: false,
        ..config
    };
    assert_eq!(store.save(&changed, 4).unwrap().revision, 5);
    assert!(store.save(&changed, 4).is_err());
    assert_eq!(store.load().unwrap().revision, 5);
}

#[test]
fn v1_settings_keep_revision_and_gain_private_notification_defaults() {
    let dir = Directory::new();
    let old = br#"{"schemaVersion":1,"revision":9,"enabled":false}"#;
    fs::write(dir.0.join("island.json"), old).unwrap();
    let config = ConfigStore::new(dir.0.clone()).load().unwrap();
    assert_eq!(config.schema_version, SCHEMA_VERSION);
    assert_eq!(config.revision, 9);
    assert!(!config.enabled);
    assert!(config.notifications.enabled);
    assert_eq!(fs::read(dir.0.join("island.json.bak")).unwrap(), old);
}

#[test]
fn v2_removes_retired_fields_and_preserves_opt_outs_and_placement() {
    let dir = Directory::new();
    let old = br##"{"schemaVersion":2,"revision":7,"hotkeys":{"toggle":"Ctrl+Alt+I"},"startup":{"closeToTray":false},"interaction":{"collapseDelayMs":400},"notifications":{"enabled":true,"qqEnabled":false,"showPreview":true},"placement":{"offsetX":72},"appearance":{"background":"#123456"}}"##;
    fs::write(dir.0.join("island.json"), old).unwrap();
    let config = ConfigStore::new(dir.0.clone()).load().unwrap();
    assert_eq!(config.revision, 7);
    assert!(!config.notifications.enabled);
    assert_eq!(config.placement.offset_x, 72.0);
    assert_eq!(config.appearance.background, "#123456");
    assert_eq!(fs::read(dir.0.join("island.json.bak")).unwrap(), old);
    let saved: Value =
        serde_json::from_slice(&fs::read(dir.0.join("island.json")).unwrap()).unwrap();
    assert_eq!(
        saved["notifications"],
        serde_json::json!({"enabled": false})
    );
    assert!(saved.get("hotkeys").is_none());
}

#[test]
fn v3_default_capsule_becomes_the_notch_without_replacing_custom_colors() {
    let dir = Directory::new();
    let old = br##"{"schemaVersion":3,"revision":5,"appearance":{"compactWidth":156,"compactHeight":44,"expandedWidth":328,"expandedHeight":168,"radius":24,"background":"#123456","border":"#34383C","opacity":0.96}}"##;
    fs::write(dir.0.join("island.json"), old).unwrap();
    let saved = ConfigStore::new(dir.0.clone()).load().unwrap();
    assert_eq!(saved.appearance.compact_width, 120.0);
    assert_eq!(saved.appearance.background, "#123456");
    assert_eq!(saved.revision, 5);
    assert_eq!(fs::read(dir.0.join("island.json.bak")).unwrap(), old);
}

#[test]
fn concurrent_writers_cannot_commit_the_same_revision_twice() {
    let dir = Directory::new();
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let save = || {
            let store = ConfigStore::new(dir.0.clone());
            barrier.wait();
            store
                .save(
                    &IslandConfig {
                        enabled: false,
                        ..Default::default()
                    },
                    0,
                )
                .is_ok()
        };
        let first = scope.spawn(save);
        let second = scope.spawn(save);
        assert_ne!(first.join().unwrap(), second.join().unwrap());
    });
}

#[test]
fn malformed_and_future_versions_are_preserved_on_load_and_save() {
    let dir = Directory::new();
    let path = dir.0.join("island.json");
    let store = ConfigStore::new(dir.0.clone());
    for old in ["{", r#"{"schemaVersion":99}"#] {
        fs::write(&path, old).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&IslandConfig::default(), 0).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), old);
    }
}
