use super::error::{NginxError, NginxResult};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

pub struct ProcessOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_nginx(binary: &Path, arguments: &[&str]) -> NginxResult<ProcessOutput> {
    run_command(binary, arguments, None, 5)
}

pub fn run_command(
    binary: &Path,
    arguments: &[&str],
    current_dir: Option<&Path>,
    timeout_seconds: u64,
) -> NginxResult<ProcessOutput> {
    let mut child = Command::new(binary)
        .args(arguments)
        .current_dir(current_dir.unwrap_or_else(|| Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| NginxError::io("unable to start nginx", error))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let status = match child
        .wait_timeout(Duration::from_secs(timeout_seconds))
        .map_err(|error| NginxError::io("unable to wait for nginx", error))?
    {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(NginxError::new(
                "NGINX_PROCESS_TIMEOUT",
                "nginx did not finish within the fixed timeout",
            ));
        }
    };

    Ok(ProcessOutput {
        success: status.success(),
        stdout: stdout_reader.join().unwrap_or_default(),
        stderr: stderr_reader.join().unwrap_or_default(),
    })
}

pub fn launch_command(
    binary: &Path,
    arguments: &[&str],
    current_dir: &Path,
) -> NginxResult<ProcessOutput> {
    Command::new(binary)
        .args(arguments)
        .current_dir(current_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| NginxError::io("unable to launch nginx", error))?;
    Ok(ProcessOutput {
        success: true,
        stdout: "nginx process launched".to_owned(),
        stderr: String::new(),
    })
}

fn read_bounded<T: Read>(stream: Option<T>) -> String {
    let mut bytes = Vec::new();
    if let Some(stream) = stream {
        let _ = stream.take(MAX_OUTPUT_BYTES as u64).read_to_end(&mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
