#[cfg(windows)]
fn main() -> Result<(), String> {
    zx_once_lib::run_island_poc()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("此示例需要 Windows 10 或 Windows 11。");
}
