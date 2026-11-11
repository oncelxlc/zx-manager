#[cfg(windows)]
#[path = "frame_probe/windows.rs"]
mod probe;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    probe::run()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("此采样程序需要 Windows 桌面。");
}
