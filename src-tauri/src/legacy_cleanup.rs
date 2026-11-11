use std::fs;
use std::io;
use std::path::Path;

const RETIRED_DATA_FILES: [&str; 3] = [
    "network-usage.sqlite3",
    "network-usage.sqlite3-wal",
    "network-usage.sqlite3-shm",
];

pub fn cleanup_retired_data(app_data_dir: &Path) {
    for file_name in RETIRED_DATA_FILES {
        let path = app_data_dir.join(file_name);
        if let Err(error) = fs::remove_file(path) {
            if error.kind() != io::ErrorKind::NotFound {
                eprintln!("failed to remove retired application data {file_name}: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::cleanup_retired_data;

    #[test]
    fn removes_retired_database_and_sqlite_sidecars() {
        let directory = tempfile::tempdir().expect("temporary directory should exist");
        for file_name in [
            "network-usage.sqlite3",
            "network-usage.sqlite3-wal",
            "network-usage.sqlite3-shm",
        ] {
            std::fs::write(directory.path().join(file_name), "retired")
                .expect("retired data should be created");
        }

        cleanup_retired_data(directory.path());
        cleanup_retired_data(directory.path());

        assert!(std::fs::read_dir(directory.path())
            .expect("temporary directory should be readable")
            .next()
            .is_none());
    }
}
