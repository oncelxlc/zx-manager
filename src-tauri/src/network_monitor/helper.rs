#[cfg(windows)]
mod windows_helper {
    use crate::network_monitor::dto::{
        AttributionQuality, NetworkMonitorWarning, NetworkPath, RawApplicationDelta,
        RawApplicationSample,
    };
    use crate::network_monitor::error::{
        NetworkMonitorError, NetworkMonitorErrorCode, NetworkMonitorResult,
    };
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use sha2::{Digest, Sha256};
    use std::collections::{HashMap, HashSet};
    use std::io;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
    use std::path::Path;
    use std::slice;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};
    use windows::core::{GUID, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, LocalFree, ERROR_CANCELLED, ERROR_PIPE_CONNECTED,
        ERROR_PIPE_LISTENING, FILETIME, GENERIC_READ, GENERIC_WRITE, HANDLE, HLOCAL, WAIT_OBJECT_0,
    };
    use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
        FwpmEngineClose0, FwpmEngineOpen0,
    };
    use windows::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        SDDL_REVISION_1,
    };
    use windows::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };
    use windows::Win32::Security::{
        GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
        TOKEN_USER,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_FIRST_PIPE_INSTANCE,
        FILE_SHARE_MODE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
    };
    use windows::Win32::System::Diagnostics::Etw::{
        CloseTrace, ControlTraceW, OpenTraceW, ProcessTrace, StartTraceW, TcpIpGuid, UdpIpGuid,
        CONTROLTRACE_HANDLE, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP,
        EVENT_TRACE_FLAG_NETWORK_TCPIP, EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES,
        EVENT_TRACE_REAL_TIME_MODE, EVENT_TRACE_SYSTEM_LOGGER_MODE, PROCESSTRACE_HANDLE,
        PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME, WNODE_FLAG_TRACED_GUID,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId,
        GetNamedPipeServerProcessId, SetNamedPipeHandleState, NAMED_PIPE_MODE, PIPE_NOWAIT,
        PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
        PIPE_WAIT,
    };
    use windows::Win32::System::Rpc::RPC_C_AUTHN_WINNT;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetProcessId, GetProcessTimes, OpenProcess, OpenProcessToken,
        QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    const PROTOCOL_VERSION: u16 = 2;
    const MAX_FRAME_SIZE: usize = 4 * 1024 * 1024;
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
    const FLOW_QUEUE_CAPACITY: usize = 32_768;
    const UNKNOWN_APPLICATION_ID: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";
    const SYSTEM_APPLICATION_ID: &str =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "camelCase")]
    enum HelperRequest {
        Prepare,
        Sample,
        Pause,
        Shutdown,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "camelCase")]
    enum HelperResponse {
        Hello {
            protocol_version: u16,
            nonce: String,
        },
        Prepared,
        Paused,
        Sample {
            applications: Vec<WireApplicationDelta>,
            lost_events: u64,
            unresolved_events: u64,
            warnings: Vec<NetworkMonitorWarning>,
        },
        Shutdown,
        Error {
            code: String,
            message: String,
        },
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WireApplicationDelta {
        application_id: String,
        display_name: String,
        network_path: NetworkPath,
        download_bytes: u64,
        upload_bytes: u64,
        quality: AttributionQuality,
    }

    impl From<WireApplicationDelta> for RawApplicationDelta {
        fn from(value: WireApplicationDelta) -> Self {
            Self {
                application_id: value.application_id,
                display_name: value.display_name,
                network_path: value.network_path,
                download_bytes: value.download_bytes,
                upload_bytes: value.upload_bytes,
                quality: value.quality,
            }
        }
    }

    impl From<RawApplicationDelta> for WireApplicationDelta {
        fn from(value: RawApplicationDelta) -> Self {
            Self {
                application_id: value.application_id,
                display_name: value.display_name,
                network_path: value.network_path,
                download_bytes: value.download_bytes,
                upload_bytes: value.upload_bytes,
                quality: value.quality,
            }
        }
    }

    struct OwnedHandle(HANDLE);

    unsafe impl Send for OwnedHandle {}

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
    }

    impl OwnedHandle {
        fn read_exact(&self, mut buffer: &mut [u8]) -> io::Result<()> {
            while !buffer.is_empty() {
                let mut read = 0_u32;
                unsafe {
                    ReadFile(self.0, Some(buffer), Some(&mut read), None).map_err(windows_to_io)?;
                }
                if read == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "named pipe closed",
                    ));
                }
                buffer = &mut buffer[read as usize..];
            }
            Ok(())
        }

        fn write_all(&self, mut buffer: &[u8]) -> io::Result<()> {
            while !buffer.is_empty() {
                let mut written = 0_u32;
                unsafe {
                    WriteFile(self.0, Some(buffer), Some(&mut written), None)
                        .map_err(windows_to_io)?;
                }
                if written == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "named pipe write returned zero",
                    ));
                }
                buffer = &buffer[written as usize..];
            }
            Ok(())
        }
    }

    struct LocalSecurityDescriptor(PSECURITY_DESCRIPTOR);

    impl Drop for LocalSecurityDescriptor {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                unsafe {
                    LocalFree(Some(HLOCAL(self.0 .0)));
                }
            }
        }
    }

    pub struct HelperClient {
        pipe: OwnedHandle,
        process: OwnedHandle,
    }

    impl HelperClient {
        pub fn start() -> NetworkMonitorResult<Self> {
            let nonce = random_nonce()?;
            let pipe_name = format!(
                r"\\.\pipe\zx-manager-network-{}-{}",
                std::process::id(),
                &nonce[..16]
            );
            let pipe_name_wide = wide(&pipe_name);
            let (descriptor, security_attributes) = pipe_security_attributes()?;
            let open_mode =
                FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX.0 | FILE_FLAG_FIRST_PIPE_INSTANCE.0);
            let pipe_mode = NAMED_PIPE_MODE(
                PIPE_TYPE_BYTE.0
                    | PIPE_READMODE_BYTE.0
                    | PIPE_NOWAIT.0
                    | PIPE_REJECT_REMOTE_CLIENTS.0,
            );
            let pipe = unsafe {
                CreateNamedPipeW(
                    PCWSTR(pipe_name_wide.as_ptr()),
                    open_mode,
                    pipe_mode,
                    PIPE_UNLIMITED_INSTANCES,
                    64 * 1024,
                    64 * 1024,
                    0,
                    Some(&security_attributes),
                )
            };
            drop(descriptor);
            if pipe.is_invalid() {
                return Err(last_windows_error(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    "failed to create the network helper pipe",
                ));
            }
            let pipe = OwnedHandle(pipe);
            let process = launch_elevated_helper(&pipe_name, &nonce)?;
            wait_for_helper_connection(&pipe, &process)?;

            let mut client_pid = 0_u32;
            unsafe {
                GetNamedPipeClientProcessId(pipe.0, &mut client_pid).map_err(|error| {
                    windows_error(
                        NetworkMonitorErrorCode::HelperDisconnected,
                        "failed to validate helper process",
                        error,
                    )
                })?;
            }
            let launched_pid = unsafe { GetProcessId(process.0) };
            validate_helper_client_pid(launched_pid, client_pid)?;

            let hello: HelperResponse = read_frame(&pipe).map_err(helper_io_error)?;
            validate_helper_hello(hello, &nonce)?;
            Ok(Self { pipe, process })
        }

        pub fn sample(&mut self) -> NetworkMonitorResult<RawApplicationSample> {
            write_frame(&self.pipe, &HelperRequest::Sample).map_err(helper_io_error)?;
            let response: HelperResponse = read_frame(&self.pipe).map_err(helper_io_error)?;
            match response {
                HelperResponse::Sample {
                    applications,
                    lost_events,
                    unresolved_events,
                    warnings,
                } => Ok(RawApplicationSample {
                    applications: applications.into_iter().map(Into::into).collect(),
                    lost_events,
                    unresolved_events,
                    warnings,
                }),
                HelperResponse::Error { code, message } => Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    format!("{code}: {message}"),
                )),
                _ => Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::ProtocolMismatch,
                    "network helper returned an unexpected response",
                )),
            }
        }

        pub fn prepare(&mut self) -> NetworkMonitorResult<()> {
            write_frame(&self.pipe, &HelperRequest::Prepare).map_err(helper_io_error)?;
            match read_frame::<HelperResponse>(&self.pipe).map_err(helper_io_error)? {
                HelperResponse::Prepared => Ok(()),
                HelperResponse::Error { code, message } => Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    format!("{code}: {message}"),
                )),
                _ => Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::ProtocolMismatch,
                    "network helper returned an unexpected prepare response",
                )),
            }
        }

        pub fn pause(&mut self) -> NetworkMonitorResult<()> {
            write_frame(&self.pipe, &HelperRequest::Pause).map_err(helper_io_error)?;
            match read_frame::<HelperResponse>(&self.pipe).map_err(helper_io_error)? {
                HelperResponse::Paused => Ok(()),
                HelperResponse::Error { code, message } => Err(NetworkMonitorError::new(
                    parse_network_error_code(&code),
                    message,
                )),
                _ => Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::ProtocolMismatch,
                    "network helper returned an unexpected pause response",
                )),
            }
        }

        fn shutdown(&mut self) {
            let _ = write_frame(&self.pipe, &HelperRequest::Shutdown);
            let _ = read_frame::<HelperResponse>(&self.pipe);
            if unsafe { WaitForSingleObject(self.process.0, 3_000) } != WAIT_OBJECT_0 {
                let _ = unsafe { TerminateProcess(self.process.0, 1) };
                let _ = unsafe { WaitForSingleObject(self.process.0, 1_000) };
            }
        }
    }

    impl Drop for HelperClient {
        fn drop(&mut self) {
            self.shutdown();
        }
    }

    fn launch_elevated_helper(pipe_name: &str, nonce: &str) -> NetworkMonitorResult<OwnedHandle> {
        let executable = std::env::current_exe().map_err(|error| {
            NetworkMonitorError::new(
                NetworkMonitorErrorCode::CollectorUnavailable,
                format!("failed to resolve current executable: {error}"),
            )
        })?;
        let executable_wide = wide(executable.as_os_str().to_string_lossy().as_ref());
        let parameters = format!(
            "--network-monitor-helper --pipe \"{pipe_name}\" --nonce {nonce} --parent {}",
            std::process::id()
        );
        let parameters_wide = wide(&parameters);
        let mut execute = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: windows::core::w!("runas"),
            lpFile: PCWSTR(executable_wide.as_ptr()),
            lpParameters: PCWSTR(parameters_wide.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        unsafe {
            ShellExecuteExW(&mut execute).map_err(|error| {
                let code = launch_failure_code(GetLastError().0);
                windows_error(code, "failed to launch elevated network helper", error)
            })?;
        }
        if execute.hProcess.is_invalid() {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::CollectorUnavailable,
                "ShellExecuteExW did not return a helper process handle",
            ));
        }
        Ok(OwnedHandle(execute.hProcess))
    }

    fn wait_for_helper_connection(
        pipe: &OwnedHandle,
        process: &OwnedHandle,
    ) -> NetworkMonitorResult<()> {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        loop {
            match unsafe { ConnectNamedPipe(pipe.0, None) } {
                Ok(()) => break,
                Err(error) => {
                    let last_error = unsafe { GetLastError() };
                    if last_error == ERROR_PIPE_CONNECTED {
                        break;
                    }
                    if last_error != ERROR_PIPE_LISTENING {
                        return Err(windows_error(
                            NetworkMonitorErrorCode::HelperDisconnected,
                            "failed while waiting for network helper",
                            error,
                        ));
                    }
                }
            }
            if unsafe { WaitForSingleObject(process.0, 0) } == WAIT_OBJECT_0 {
                return Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::HelperDisconnected,
                    "network helper exited before connecting",
                ));
            }
            if Instant::now() >= deadline {
                return Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::HelperDisconnected,
                    "timed out waiting for network helper",
                ));
            }
            thread::sleep(Duration::from_millis(50));
        }
        let blocking_mode = NAMED_PIPE_MODE(PIPE_READMODE_BYTE.0 | PIPE_WAIT.0);
        unsafe {
            SetNamedPipeHandleState(pipe.0, Some(&blocking_mode), None, None).map_err(|error| {
                windows_error(
                    NetworkMonitorErrorCode::HelperDisconnected,
                    "failed to switch helper pipe to blocking mode",
                    error,
                )
            })
        }
    }

    fn pipe_security_attributes(
    ) -> NetworkMonitorResult<(LocalSecurityDescriptor, SECURITY_ATTRIBUTES)> {
        let user_sid = current_user_sid_string()?;
        let sddl = wide(&format!("D:P(A;;GA;;;SY)(A;;GA;;;{user_sid})"));
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
            .map_err(|error| {
                windows_error(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    "failed to build the network helper pipe ACL",
                    error,
                )
            })?;
        }
        let owner = LocalSecurityDescriptor(descriptor);
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        Ok((owner, attributes))
    }

    fn current_user_sid_string() -> NetworkMonitorResult<String> {
        let mut token = HANDLE::default();
        unsafe {
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).map_err(|error| {
                windows_error(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    "failed to open the current process token",
                    error,
                )
            })?;
        }
        let token = OwnedHandle(token);
        let mut required = 0_u32;
        let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut required) };
        if required == 0 {
            return Err(last_windows_error(
                NetworkMonitorErrorCode::CollectorUnavailable,
                "failed to size the current user token",
            ));
        }
        let mut buffer = vec![0_u8; required as usize];
        unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                required,
                &mut required,
            )
            .map_err(|error| {
                windows_error(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    "failed to read the current user token",
                    error,
                )
            })?;
        }
        let token_user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
        let mut sid_text = PWSTR::null();
        unsafe {
            ConvertSidToStringSidW(token_user.User.Sid, &mut sid_text).map_err(|error| {
                windows_error(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    "failed to serialize the current user SID",
                    error,
                )
            })?;
        }
        let result = unsafe { sid_text.to_string() }.map_err(|error| {
            NetworkMonitorError::new(
                NetworkMonitorErrorCode::CollectorUnavailable,
                format!("failed to decode the current user SID: {error}"),
            )
        });
        unsafe {
            LocalFree(Some(HLOCAL(sid_text.0.cast())));
        }
        result
    }

    fn random_nonce() -> NetworkMonitorResult<String> {
        let mut nonce = [0_u8; 32];
        let status = unsafe { BCryptGenRandom(None, &mut nonce, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
        if status.0 < 0 {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::CollectorUnavailable,
                format!("BCryptGenRandom failed with status {}", status.0),
            ));
        }
        Ok(nonce.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    fn write_frame<T: Serialize>(pipe: &OwnedHandle, value: &T) -> io::Result<()> {
        let payload = serde_json::to_vec(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if payload.len() > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "helper frame is too large",
            ));
        }
        pipe.write_all(&(payload.len() as u32).to_le_bytes())?;
        pipe.write_all(&payload)
    }

    fn read_frame<T: DeserializeOwned>(pipe: &OwnedHandle) -> io::Result<T> {
        let mut length = [0_u8; 4];
        pipe.read_exact(&mut length)?;
        let length = u32::from_le_bytes(length) as usize;
        if length > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "helper frame exceeds the maximum size",
            ));
        }
        let mut payload = vec![0_u8; length];
        pipe.read_exact(&mut payload)?;
        serde_json::from_slice(&payload)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub fn try_run_from_args() -> bool {
        let arguments = std::env::args().collect::<Vec<_>>();
        if !arguments
            .iter()
            .any(|argument| argument == "--network-monitor-helper")
        {
            return false;
        }
        let result = run_helper(&arguments);
        if result.is_err() {
            std::process::exit(2);
        }
        true
    }

    fn run_helper(arguments: &[String]) -> NetworkMonitorResult<()> {
        let pipe_name = argument_value(arguments, "--pipe")?;
        let nonce = argument_value(arguments, "--nonce")?;
        let parent_pid = argument_value(arguments, "--parent")?
            .parse::<u32>()
            .map_err(|_| {
                NetworkMonitorError::new(
                    NetworkMonitorErrorCode::InvalidRequest,
                    "helper parent PID was invalid",
                )
            })?;
        let pipe = connect_to_parent(pipe_name)?;
        let mut server_pid = 0_u32;
        unsafe {
            GetNamedPipeServerProcessId(pipe.0, &mut server_pid).map_err(|error| {
                windows_error(
                    NetworkMonitorErrorCode::ProtocolMismatch,
                    "failed to validate the helper pipe server",
                    error,
                )
            })?;
        }
        validate_helper_server_pid(parent_pid, server_pid)?;
        write_frame(
            &pipe,
            &HelperResponse::Hello {
                protocol_version: PROTOCOL_VERSION,
                nonce: nonce.to_owned(),
            },
        )
        .map_err(helper_io_error)?;

        let mut runtime: Option<EtwRuntime> = None;
        while let Ok(request) = read_frame::<HelperRequest>(&pipe) {
            match request {
                HelperRequest::Prepare => match prepare_wfp_engine() {
                    Ok(()) => {
                        write_frame(&pipe, &HelperResponse::Prepared).map_err(helper_io_error)?
                    }
                    Err(error) => write_frame(
                        &pipe,
                        &HelperResponse::Error {
                            code: network_error_code_name(error.code).to_owned(),
                            message: error.message,
                        },
                    )
                    .map_err(helper_io_error)?,
                },
                HelperRequest::Sample => {
                    let sample = get_or_start_runtime(&mut runtime, EtwRuntime::start)?.snapshot();
                    write_frame(
                        &pipe,
                        &HelperResponse::Sample {
                            applications: sample.applications.into_iter().map(Into::into).collect(),
                            lost_events: sample.lost_events,
                            unresolved_events: sample.unresolved_events,
                            warnings: sample.warnings,
                        },
                    )
                    .map_err(helper_io_error)?;
                }
                HelperRequest::Pause => {
                    stop_runtime(&mut runtime, EtwRuntime::stop);
                    write_frame(&pipe, &HelperResponse::Paused).map_err(helper_io_error)?;
                }
                HelperRequest::Shutdown => {
                    let _ = write_frame(&pipe, &HelperResponse::Shutdown);
                    break;
                }
            }
        }
        stop_runtime(&mut runtime, EtwRuntime::stop);
        Ok(())
    }

    fn get_or_start_runtime<T, E>(
        runtime: &mut Option<T>,
        start: impl FnOnce() -> Result<T, E>,
    ) -> Result<&mut T, E> {
        if runtime.is_none() {
            *runtime = Some(start()?);
        }
        Ok(runtime.as_mut().expect("helper runtime initialized"))
    }

    fn stop_runtime<T>(runtime: &mut Option<T>, stop: impl FnOnce(T)) {
        if let Some(runtime) = runtime.take() {
            stop(runtime);
        }
    }

    fn prepare_wfp_engine() -> NetworkMonitorResult<()> {
        let mut engine = HANDLE::default();
        unsafe {
            let status = FwpmEngineOpen0(None, RPC_C_AUTHN_WINNT, None, None, &mut engine);
            if status != 0 {
                return Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    format!("failed to open the Windows Filtering Platform engine: {status}"),
                ));
            }
            let status = FwpmEngineClose0(engine);
            if status != 0 {
                return Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    format!("failed to close the Windows Filtering Platform engine: {status}"),
                ));
            }
        }
        Ok(())
    }

    fn connect_to_parent(pipe_name: &str) -> NetworkMonitorResult<OwnedHandle> {
        let pipe_name = wide(pipe_name);
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        loop {
            let handle = unsafe {
                CreateFileW(
                    PCWSTR(pipe_name.as_ptr()),
                    GENERIC_READ.0 | GENERIC_WRITE.0,
                    FILE_SHARE_MODE(0),
                    None,
                    OPEN_EXISTING,
                    FILE_FLAGS_AND_ATTRIBUTES(0),
                    None,
                )
            };
            match handle {
                Ok(handle) => return Ok(OwnedHandle(handle)),
                Err(error) if Instant::now() < deadline => {
                    let _ = error;
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => {
                    return Err(windows_error(
                        NetworkMonitorErrorCode::HelperDisconnected,
                        "failed to connect to the parent network pipe",
                        error,
                    ));
                }
            }
        }
    }

    fn argument_value<'a>(arguments: &'a [String], key: &str) -> NetworkMonitorResult<&'a str> {
        arguments
            .iter()
            .position(|argument| argument == key)
            .and_then(|index| arguments.get(index + 1))
            .map(String::as_str)
            .ok_or_else(|| {
                NetworkMonitorError::new(
                    NetworkMonitorErrorCode::InvalidRequest,
                    format!("missing helper argument {key}"),
                )
            })
    }

    fn validate_helper_client_pid(launched_pid: u32, client_pid: u32) -> NetworkMonitorResult<()> {
        if launched_pid != 0 && client_pid == launched_pid {
            return Ok(());
        }
        Err(NetworkMonitorError::new(
            NetworkMonitorErrorCode::ProtocolMismatch,
            "network helper pipe client PID did not match the launched process",
        ))
    }

    fn validate_helper_server_pid(
        expected_parent_pid: u32,
        server_pid: u32,
    ) -> NetworkMonitorResult<()> {
        if expected_parent_pid != 0 && server_pid == expected_parent_pid {
            return Ok(());
        }
        Err(NetworkMonitorError::new(
            NetworkMonitorErrorCode::ProtocolMismatch,
            "helper pipe server PID did not match the requested parent",
        ))
    }

    fn validate_helper_hello(
        response: HelperResponse,
        expected_nonce: &str,
    ) -> NetworkMonitorResult<()> {
        match response {
            HelperResponse::Hello {
                protocol_version,
                nonce,
            } if protocol_version == PROTOCOL_VERSION && nonce == expected_nonce => Ok(()),
            HelperResponse::Error { code, message } => Err(NetworkMonitorError::new(
                parse_network_error_code(&code),
                message,
            )),
            _ => Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::ProtocolMismatch,
                "network helper handshake failed",
            )),
        }
    }

    fn launch_failure_code(last_error: u32) -> NetworkMonitorErrorCode {
        if last_error == ERROR_CANCELLED.0 {
            NetworkMonitorErrorCode::ElevationCancelled
        } else {
            NetworkMonitorErrorCode::CollectorUnavailable
        }
    }

    fn network_error_code_name(code: NetworkMonitorErrorCode) -> &'static str {
        match code {
            NetworkMonitorErrorCode::InvalidRequest => "invalidRequest",
            NetworkMonitorErrorCode::UnsupportedPlatform => "unsupportedPlatform",
            NetworkMonitorErrorCode::ElevationCancelled => "elevationCancelled",
            NetworkMonitorErrorCode::HelperDisconnected => "helperDisconnected",
            NetworkMonitorErrorCode::ProtocolMismatch => "protocolMismatch",
            NetworkMonitorErrorCode::CollectorUnavailable => "collectorUnavailable",
            NetworkMonitorErrorCode::StorageUnavailable => "storageUnavailable",
            NetworkMonitorErrorCode::Internal => "internal",
        }
    }

    fn parse_network_error_code(code: &str) -> NetworkMonitorErrorCode {
        match code {
            "invalidRequest" => NetworkMonitorErrorCode::InvalidRequest,
            "unsupportedPlatform" => NetworkMonitorErrorCode::UnsupportedPlatform,
            "elevationCancelled" => NetworkMonitorErrorCode::ElevationCancelled,
            "helperDisconnected" => NetworkMonitorErrorCode::HelperDisconnected,
            "protocolMismatch" => NetworkMonitorErrorCode::ProtocolMismatch,
            "storageUnavailable" => NetworkMonitorErrorCode::StorageUnavailable,
            "internal" => NetworkMonitorErrorCode::Internal,
            _ => NetworkMonitorErrorCode::CollectorUnavailable,
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum Direction {
        Download,
        Upload,
    }

    #[derive(Debug)]
    struct FlowEvent {
        pid: u32,
        size: u64,
        direction: Direction,
        remote: Option<SocketAddr>,
    }

    struct CallbackContext {
        sender: SyncSender<FlowEvent>,
        lost_events: Arc<AtomicU64>,
        last_etw_lost: AtomicU64,
    }

    unsafe extern "system" fn event_record_callback(record: *mut EVENT_RECORD) {
        if record.is_null() {
            return;
        }
        let record = unsafe { &*record };
        let context = record.UserContext.cast::<CallbackContext>();
        if context.is_null() {
            return;
        }
        let context = unsafe { &*context };
        let provider = record.EventHeader.ProviderId;
        if provider != TcpIpGuid && provider != UdpIpGuid {
            return;
        }
        let opcode = u32::from(record.EventHeader.EventDescriptor.Opcode);
        let (direction, ipv6) = match opcode {
            10 => (Direction::Upload, false),
            11 => (Direction::Download, false),
            26 => (Direction::Upload, true),
            27 => (Direction::Download, true),
            _ => return,
        };
        let data = unsafe {
            slice::from_raw_parts(
                record.UserData.cast::<u8>(),
                usize::from(record.UserDataLength),
            )
        };
        let Some((pid, size, remote)) = decode_network_payload(data, direction, ipv6) else {
            context.lost_events.fetch_add(1, Ordering::Relaxed);
            return;
        };
        match context.sender.try_send(FlowEvent {
            pid,
            size,
            direction,
            remote,
        }) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                context.lost_events.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    unsafe extern "system" fn buffer_callback(logfile: *mut EVENT_TRACE_LOGFILEW) -> u32 {
        if logfile.is_null() {
            return 1;
        }
        let logfile = unsafe { &*logfile };
        let context = logfile.Context.cast::<CallbackContext>();
        if context.is_null() {
            return 1;
        }
        let context = unsafe { &*context };
        let current = u64::from(logfile.EventsLost);
        let previous = context.last_etw_lost.swap(current, Ordering::AcqRel);
        if current > previous {
            context
                .lost_events
                .fetch_add(current - previous, Ordering::Relaxed);
        }
        1
    }

    fn decode_network_payload(
        data: &[u8],
        direction: Direction,
        ipv6: bool,
    ) -> Option<(u32, u64, Option<SocketAddr>)> {
        if data.len() < 8 {
            return None;
        }
        let pid = u32::from_ne_bytes(data[0..4].try_into().ok()?);
        let size = u64::from(u32::from_ne_bytes(data[4..8].try_into().ok()?));
        let remote = if ipv6 {
            if data.len() < 44 {
                None
            } else {
                let address_offset = match direction {
                    Direction::Upload => 8,
                    Direction::Download => 24,
                };
                let port_offset = match direction {
                    Direction::Upload => 40,
                    Direction::Download => 42,
                };
                let address = Ipv6Addr::from(
                    <[u8; 16]>::try_from(&data[address_offset..address_offset + 16]).ok()?,
                );
                let port = u16::from_be_bytes(data[port_offset..port_offset + 2].try_into().ok()?);
                Some(SocketAddr::new(IpAddr::V6(address), port))
            }
        } else if data.len() < 20 {
            None
        } else {
            let address_offset = match direction {
                Direction::Upload => 8,
                Direction::Download => 12,
            };
            let port_offset = match direction {
                Direction::Upload => 16,
                Direction::Download => 18,
            };
            let address = Ipv4Addr::new(
                data[address_offset],
                data[address_offset + 1],
                data[address_offset + 2],
                data[address_offset + 3],
            );
            let port = u16::from_be_bytes(data[port_offset..port_offset + 2].try_into().ok()?);
            Some(SocketAddr::new(IpAddr::V4(address), port))
        };
        Some((pid, size, remote))
    }

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct AggregateKey {
        application_id: String,
        display_name: String,
        network_path: NetworkPath,
        quality: AttributionQuality,
    }

    #[derive(Default)]
    struct AggregateBytes {
        download_bytes: u64,
        upload_bytes: u64,
    }

    #[derive(Default)]
    struct AggregatedState {
        applications: HashMap<AggregateKey, AggregateBytes>,
        unresolved_events: u64,
    }

    struct EtwRuntime {
        state: Arc<Mutex<AggregatedState>>,
        lost_events: Arc<AtomicU64>,
        stop: Arc<AtomicBool>,
        session: EtwSession,
        worker: Option<JoinHandle<()>>,
    }

    impl EtwRuntime {
        fn start() -> NetworkMonitorResult<Self> {
            let (sender, receiver) = mpsc::sync_channel(FLOW_QUEUE_CAPACITY);
            let state = Arc::new(Mutex::new(AggregatedState::default()));
            let lost_events = Arc::new(AtomicU64::new(0));
            let stop = Arc::new(AtomicBool::new(false));
            let worker_state = Arc::clone(&state);
            let worker_stop = Arc::clone(&stop);
            let worker = thread::Builder::new()
                .name("network-helper-aggregator".to_owned())
                .spawn(move || aggregate_flows(receiver, worker_state, worker_stop))
                .map_err(|error| {
                    NetworkMonitorError::new(
                        NetworkMonitorErrorCode::CollectorUnavailable,
                        format!("failed to start network aggregation worker: {error}"),
                    )
                })?;
            match EtwSession::start(sender, Arc::clone(&lost_events)) {
                Ok(session) => Ok(Self {
                    state,
                    lost_events,
                    stop,
                    session,
                    worker: Some(worker),
                }),
                Err(error) => {
                    stop.store(true, Ordering::Release);
                    let _ = worker.join();
                    Err(error)
                }
            }
        }

        fn snapshot(&self) -> RawApplicationSample {
            let mut state = lock(&self.state);
            let applications = std::mem::take(&mut state.applications)
                .into_iter()
                .map(|(key, bytes)| RawApplicationDelta {
                    application_id: key.application_id,
                    display_name: key.display_name,
                    network_path: key.network_path,
                    download_bytes: bytes.download_bytes,
                    upload_bytes: bytes.upload_bytes,
                    quality: key.quality,
                })
                .collect();
            let unresolved_events = std::mem::take(&mut state.unresolved_events);
            let lost_events = self.lost_events.swap(0, Ordering::AcqRel);
            let mut warnings = Vec::new();
            if lost_events > 0 {
                warnings.push(NetworkMonitorWarning::new("etwEventsLost"));
            }
            if unresolved_events > 0 {
                warnings.push(NetworkMonitorWarning::new("applicationIdentityUnresolved"));
            }
            RawApplicationSample {
                applications,
                lost_events,
                unresolved_events,
                warnings,
            }
        }

        fn stop(mut self) {
            self.session.stop();
            self.stop.store(true, Ordering::Release);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    fn aggregate_flows(
        receiver: Receiver<FlowEvent>,
        state: Arc<Mutex<AggregatedState>>,
        stop: Arc<AtomicBool>,
    ) {
        let mut resolver = ProcessResolver::default();
        let mut classifier = ProxyClassifier::default();
        while !stop.load(Ordering::Acquire) {
            let flow = match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(flow) => flow,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            classifier.refresh_if_needed();
            let path = classifier.classify(flow.remote);
            if flow
                .remote
                .is_some_and(|remote| remote.ip().is_loopback() && path != NetworkPath::Proxy)
            {
                continue;
            }
            let identity = resolver.resolve(flow.pid);
            let unresolved = identity.application_id == UNKNOWN_APPLICATION_ID;
            let quality = if unresolved || path == NetworkPath::Unknown {
                AttributionQuality::Partial
            } else {
                AttributionQuality::Exact
            };
            let key = AggregateKey {
                application_id: identity.application_id,
                display_name: identity.display_name,
                network_path: path,
                quality,
            };
            let mut state = lock(&state);
            if unresolved {
                state.unresolved_events = state.unresolved_events.saturating_add(1);
            }
            let bytes = state.applications.entry(key).or_default();
            match flow.direction {
                Direction::Download => {
                    bytes.download_bytes = bytes.download_bytes.saturating_add(flow.size);
                }
                Direction::Upload => {
                    bytes.upload_bytes = bytes.upload_bytes.saturating_add(flow.size);
                }
            }
        }
    }

    struct EtwSession {
        control_handle: CONTROLTRACE_HANDLE,
        trace_handle: PROCESSTRACE_HANDLE,
        properties: Vec<usize>,
        consumer: Option<JoinHandle<()>>,
    }

    impl EtwSession {
        fn start(
            sender: SyncSender<FlowEvent>,
            lost_events: Arc<AtomicU64>,
        ) -> NetworkMonitorResult<Self> {
            let session_name = format!(
                "ZxManager.Network.{}.{}",
                std::process::id(),
                UtcTimestamp::now()
            );
            let session_name_wide = wide(&session_name);
            let session_guid = random_session_guid()?;
            let mut properties = trace_properties_buffer(&session_name_wide, session_guid);
            let properties_ptr = properties.as_mut_ptr().cast::<EVENT_TRACE_PROPERTIES>();
            let mut control_handle = CONTROLTRACE_HANDLE::default();
            let status = unsafe {
                StartTraceW(
                    &mut control_handle,
                    PCWSTR(session_name_wide.as_ptr()),
                    properties_ptr,
                )
            };
            if status.0 != 0 {
                return Err(NetworkMonitorError::new(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    format!("StartTraceW failed with code {}", status.0),
                ));
            }

            let context = Box::new(CallbackContext {
                sender,
                lost_events,
                last_etw_lost: AtomicU64::new(0),
            });
            let context_ptr = Box::into_raw(context);
            let context_address = context_ptr as usize;
            let mut logfile = EVENT_TRACE_LOGFILEW {
                LoggerName: PWSTR(session_name_wide.as_ptr().cast_mut()),
                BufferCallback: Some(buffer_callback),
                Context: context_ptr.cast(),
                ..Default::default()
            };
            logfile.Anonymous1.ProcessTraceMode =
                PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
            logfile.Anonymous2.EventRecordCallback = Some(event_record_callback);
            let trace_handle = unsafe { OpenTraceW(&mut logfile) };
            if trace_handle.Value == u64::MAX {
                unsafe {
                    let _ = ControlTraceW(
                        control_handle,
                        PCWSTR::null(),
                        properties_ptr,
                        EVENT_TRACE_CONTROL_STOP,
                    );
                    drop(Box::from_raw(context_ptr));
                }
                return Err(last_windows_error(
                    NetworkMonitorErrorCode::CollectorUnavailable,
                    "OpenTraceW failed for the network helper",
                ));
            }
            let consumer_context_address = context_address;
            let consumer = thread::Builder::new()
                .name("network-helper-etw".to_owned())
                .spawn(move || {
                    let _ = unsafe { ProcessTrace(&[trace_handle], None, None) };
                    unsafe {
                        drop(Box::from_raw(
                            consumer_context_address as *mut CallbackContext,
                        ));
                    }
                })
                .map_err(|error| {
                    unsafe {
                        let _ = CloseTrace(trace_handle);
                        let _ = ControlTraceW(
                            control_handle,
                            PCWSTR::null(),
                            properties_ptr,
                            EVENT_TRACE_CONTROL_STOP,
                        );
                        drop(Box::from_raw(context_address as *mut CallbackContext));
                    }
                    NetworkMonitorError::new(
                        NetworkMonitorErrorCode::CollectorUnavailable,
                        format!("failed to start the ETW consumer: {error}"),
                    )
                })?;
            Ok(Self {
                control_handle,
                trace_handle,
                properties,
                consumer: Some(consumer),
            })
        }

        fn stop(&mut self) {
            unsafe {
                let properties = self
                    .properties
                    .as_mut_ptr()
                    .cast::<EVENT_TRACE_PROPERTIES>();
                let _ = ControlTraceW(
                    self.control_handle,
                    PCWSTR::null(),
                    properties,
                    EVENT_TRACE_CONTROL_STOP,
                );
                let _ = CloseTrace(self.trace_handle);
            }
            if let Some(consumer) = self.consumer.take() {
                let _ = consumer.join();
            }
        }
    }

    impl Drop for EtwSession {
        fn drop(&mut self) {
            if self.consumer.is_some() {
                self.stop();
            }
        }
    }

    fn trace_properties_buffer(session_name: &[u16], session_guid: GUID) -> Vec<usize> {
        let property_size = std::mem::size_of::<EVENT_TRACE_PROPERTIES>();
        let name_size = std::mem::size_of_val(session_name);
        let byte_size = property_size + name_size;
        let word_size = std::mem::size_of::<usize>();
        let mut buffer = vec![0_usize; byte_size.div_ceil(word_size)];
        let properties = unsafe { &mut *buffer.as_mut_ptr().cast::<EVENT_TRACE_PROPERTIES>() };
        properties.Wnode.BufferSize = byte_size as u32;
        properties.Wnode.Guid = session_guid;
        properties.Wnode.ClientContext = 1;
        properties.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        properties.BufferSize = 64;
        properties.MinimumBuffers = 4;
        properties.MaximumBuffers = 64;
        properties.LogFileMode = EVENT_TRACE_REAL_TIME_MODE | EVENT_TRACE_SYSTEM_LOGGER_MODE;
        properties.FlushTimer = 1;
        properties.EnableFlags = EVENT_TRACE_FLAG_NETWORK_TCPIP;
        properties.LoggerNameOffset = property_size as u32;
        unsafe {
            std::ptr::copy_nonoverlapping(
                session_name.as_ptr().cast::<u8>(),
                buffer.as_mut_ptr().cast::<u8>().add(property_size),
                name_size,
            );
        }
        buffer
    }

    fn random_session_guid() -> NetworkMonitorResult<GUID> {
        let mut bytes = [0_u8; 16];
        let status = unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
        if status.0 != 0 {
            return Err(NetworkMonitorError::new(
                NetworkMonitorErrorCode::CollectorUnavailable,
                format!("BCryptGenRandom failed with status {}", status.0),
            ));
        }
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(GUID::from_u128(u128::from_be_bytes(bytes)))
    }

    #[derive(Clone)]
    struct ProcessIdentity {
        application_id: String,
        display_name: String,
        creation_time: u64,
    }

    #[derive(Default)]
    struct ProcessResolver {
        cache: HashMap<u32, ProcessIdentity>,
    }

    impl ProcessResolver {
        fn resolve(&mut self, pid: u32) -> ProcessIdentity {
            if pid == 0 || pid == 4 {
                return ProcessIdentity {
                    application_id: SYSTEM_APPLICATION_ID.to_owned(),
                    display_name: "System".to_owned(),
                    creation_time: 0,
                };
            }
            let Some((handle, creation_time)) = open_process_with_creation_time(pid) else {
                return unknown_process_identity();
            };
            if let Some(identity) = self.cached_identity(pid, creation_time) {
                return identity;
            }
            match resolve_process_identity(handle, creation_time) {
                Some(identity) => {
                    self.cache.insert(pid, identity.clone());
                    identity
                }
                None => unknown_process_identity(),
            }
        }

        fn cached_identity(&self, pid: u32, creation_time: u64) -> Option<ProcessIdentity> {
            self.cache
                .get(&pid)
                .filter(|identity| identity.creation_time == creation_time)
                .cloned()
        }
    }

    fn unknown_process_identity() -> ProcessIdentity {
        ProcessIdentity {
            application_id: UNKNOWN_APPLICATION_ID.to_owned(),
            display_name: "Unknown application".to_owned(),
            creation_time: 0,
        }
    }

    fn open_process_with_creation_time(pid: u32) -> Option<(OwnedHandle, u64)> {
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };
        let handle = OwnedHandle(handle);
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe {
            GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user).ok()?;
        }
        let creation_time =
            (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime);
        Some((handle, creation_time))
    }

    fn resolve_process_identity(
        handle: OwnedHandle,
        creation_time: u64,
    ) -> Option<ProcessIdentity> {
        let mut path_buffer = vec![0_u16; 32_768];
        let mut length = path_buffer.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                handle.0,
                PROCESS_NAME_WIN32,
                PWSTR(path_buffer.as_mut_ptr()),
                &mut length,
            )
            .ok()?;
        }
        let path = String::from_utf16_lossy(&path_buffer[..length as usize]);
        Some(identity_from_path(&path, creation_time))
    }

    fn identity_from_path(path: &str, creation_time: u64) -> ProcessIdentity {
        let normalized = path.replace('/', "\\").to_lowercase();
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let application_id = format!("{:x}", hasher.finalize());
        let display_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Unknown application")
            .to_owned();
        ProcessIdentity {
            application_id,
            display_name,
            creation_time,
        }
    }

    #[derive(Default)]
    struct ProxyClassifier {
        endpoints: HashSet<SocketAddr>,
        automatic_proxy_enabled: bool,
        resolution_failed: bool,
        refreshed_at: Option<Instant>,
    }

    impl ProxyClassifier {
        fn refresh_if_needed(&mut self) {
            if self
                .refreshed_at
                .is_some_and(|time| time.elapsed() < Duration::from_secs(60))
            {
                return;
            }
            *self = read_proxy_classifier();
        }

        fn classify(&self, remote: Option<SocketAddr>) -> NetworkPath {
            let Some(remote) = remote else {
                return NetworkPath::Unknown;
            };
            if self.endpoints.contains(&remote) {
                return NetworkPath::Proxy;
            }
            if self.automatic_proxy_enabled || self.resolution_failed {
                NetworkPath::Unknown
            } else {
                NetworkPath::Direct
            }
        }
    }

    fn read_proxy_classifier() -> ProxyClassifier {
        let mut classifier = ProxyClassifier {
            refreshed_at: Some(Instant::now()),
            ..ProxyClassifier::default()
        };
        let Ok(settings) = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        else {
            classifier.resolution_failed = true;
            return classifier;
        };
        let manual_proxy_enabled = settings.get_value::<u32, _>("ProxyEnable").unwrap_or(0) != 0;
        let pac_enabled = settings
            .get_value::<String, _>("AutoConfigURL")
            .is_ok_and(|value| !value.trim().is_empty());
        let auto_detect = settings.get_value::<u32, _>("AutoDetect").unwrap_or(0) != 0;
        classifier.automatic_proxy_enabled = pac_enabled || auto_detect;
        if manual_proxy_enabled {
            let value = settings
                .get_value::<String, _>("ProxyServer")
                .unwrap_or_default();
            for specification in proxy_specifications(&value) {
                match specification.to_socket_addrs() {
                    Ok(addresses) => classifier.endpoints.extend(addresses),
                    Err(_) => classifier.resolution_failed = true,
                }
            }
            if classifier.endpoints.is_empty() && !value.trim().is_empty() {
                classifier.resolution_failed = true;
            }
        }
        classifier
    }

    fn proxy_specifications(value: &str) -> Vec<String> {
        value
            .split(';')
            .filter_map(|entry| {
                let entry = entry
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or(entry)
                    .trim()
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .trim_start_matches("socks://");
                (!entry.is_empty() && entry.rsplit_once(':').is_some()).then(|| entry.to_owned())
            })
            .collect()
    }

    struct UtcTimestamp;

    impl UtcTimestamp {
        fn now() -> i64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .min(i64::MAX as u128) as i64
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn windows_to_io(error: windows::core::Error) -> io::Error {
        io::Error::from_raw_os_error(error.code().0)
    }

    fn helper_io_error(error: io::Error) -> NetworkMonitorError {
        NetworkMonitorError::new(
            NetworkMonitorErrorCode::HelperDisconnected,
            error.to_string(),
        )
    }

    fn windows_error(
        code: NetworkMonitorErrorCode,
        context: &str,
        error: windows::core::Error,
    ) -> NetworkMonitorError {
        NetworkMonitorError::new(code, format!("{context}: {error}"))
    }

    fn last_windows_error(code: NetworkMonitorErrorCode, context: &str) -> NetworkMonitorError {
        NetworkMonitorError::new(
            code,
            format!("{context}: Windows error {}", unsafe { GetLastError().0 }),
        )
    }

    pub use HelperClient as Client;

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn decodes_ipv4_and_ipv6_payload_pid_size_and_remote_endpoint() {
            let mut ipv4 = vec![0_u8; 20];
            ipv4[0..4].copy_from_slice(&42_u32.to_ne_bytes());
            ipv4[4..8].copy_from_slice(&512_u32.to_ne_bytes());
            ipv4[8..12].copy_from_slice(&[1, 2, 3, 4]);
            ipv4[16..18].copy_from_slice(&443_u16.to_be_bytes());
            let decoded = decode_network_payload(&ipv4, Direction::Upload, false).unwrap();
            assert_eq!(decoded.0, 42);
            assert_eq!(decoded.1, 512);
            assert_eq!(decoded.2.unwrap().to_string(), "1.2.3.4:443");

            let mut ipv6 = vec![0_u8; 44];
            ipv6[0..4].copy_from_slice(&7_u32.to_ne_bytes());
            ipv6[4..8].copy_from_slice(&64_u32.to_ne_bytes());
            ipv6[24..40].copy_from_slice(&Ipv6Addr::LOCALHOST.octets());
            ipv6[42..44].copy_from_slice(&8080_u16.to_be_bytes());
            let decoded = decode_network_payload(&ipv6, Direction::Download, true).unwrap();
            assert_eq!(decoded.0, 7);
            assert_eq!(decoded.2.unwrap().port(), 8080);
        }

        #[test]
        fn system_logger_uses_a_private_session_guid() {
            let session_guid = GUID::from_u128(0x12345678_1234_5678_90ab_1234567890ab);
            let mut buffer = trace_properties_buffer(&wide("ZxManager.Test"), session_guid);
            let properties = unsafe { &*buffer.as_mut_ptr().cast::<EVENT_TRACE_PROPERTIES>() };
            assert_eq!(properties.Wnode.Guid, session_guid);
            assert_ne!(
                properties.Wnode.Guid,
                windows::Win32::System::Diagnostics::Etw::SystemTraceControlGuid
            );
            assert_ne!(properties.LogFileMode & EVENT_TRACE_SYSTEM_LOGGER_MODE, 0);
            assert_eq!(properties.EnableFlags, EVENT_TRACE_FLAG_NETWORK_TCPIP);
        }

        #[test]
        fn path_hash_does_not_expose_the_full_executable_path() {
            let identity = identity_from_path(r"C:\Users\Alice\App\browser.exe", 1);
            assert_eq!(identity.application_id.len(), 64);
            assert_eq!(identity.display_name, "browser.exe");
            assert!(!identity.application_id.contains("Alice"));
        }

        #[test]
        fn proxy_specification_parser_handles_per_protocol_and_single_values() {
            assert_eq!(
                proxy_specifications("http=proxy.test:8080;https=secure.test:8443"),
                ["proxy.test:8080", "secure.test:8443"]
            );
            assert_eq!(proxy_specifications("127.0.0.1:7890"), ["127.0.0.1:7890"]);
        }

        #[test]
        fn handshake_rejects_wrong_pid_nonce_and_protocol() {
            assert!(validate_helper_client_pid(42, 42).is_ok());
            assert_eq!(
                validate_helper_client_pid(42, 7).unwrap_err().code,
                NetworkMonitorErrorCode::ProtocolMismatch
            );
            assert!(validate_helper_server_pid(42, 42).is_ok());
            assert_eq!(
                validate_helper_server_pid(42, 7).unwrap_err().code,
                NetworkMonitorErrorCode::ProtocolMismatch
            );
            assert!(validate_helper_hello(
                HelperResponse::Hello {
                    protocol_version: PROTOCOL_VERSION,
                    nonce: "expected".to_owned(),
                },
                "expected",
            )
            .is_ok());
            assert_eq!(
                validate_helper_hello(
                    HelperResponse::Hello {
                        protocol_version: PROTOCOL_VERSION,
                        nonce: "wrong".to_owned(),
                    },
                    "expected",
                )
                .unwrap_err()
                .code,
                NetworkMonitorErrorCode::ProtocolMismatch
            );
            assert_eq!(
                validate_helper_hello(
                    HelperResponse::Hello {
                        protocol_version: PROTOCOL_VERSION + 1,
                        nonce: "expected".to_owned(),
                    },
                    "expected",
                )
                .unwrap_err()
                .code,
                NetworkMonitorErrorCode::ProtocolMismatch
            );
        }

        #[test]
        fn pause_messages_use_the_version_two_protocol() {
            assert_eq!(PROTOCOL_VERSION, 2);
            assert_eq!(
                serde_json::to_value(HelperRequest::Pause).unwrap()["type"],
                "pause"
            );
            assert_eq!(
                serde_json::to_value(HelperResponse::Paused).unwrap()["type"],
                "paused"
            );
        }

        #[test]
        fn pause_is_idempotent_and_sampling_restarts_the_runtime() {
            let mut runtime = None;
            let mut starts = 0;
            let mut stops = 0;

            let first = get_or_start_runtime(&mut runtime, || {
                starts += 1;
                Ok::<_, ()>(starts)
            })
            .unwrap();
            assert_eq!(*first, 1);

            stop_runtime(&mut runtime, |_| stops += 1);
            stop_runtime(&mut runtime, |_| stops += 1);
            assert_eq!(stops, 1);

            let second = get_or_start_runtime(&mut runtime, || {
                starts += 1;
                Ok::<_, ()>(starts)
            })
            .unwrap();
            assert_eq!(*second, 2);
        }

        #[test]
        fn helper_startup_and_uac_failures_keep_structured_codes() {
            assert_eq!(
                launch_failure_code(ERROR_CANCELLED.0),
                NetworkMonitorErrorCode::ElevationCancelled
            );
            assert_eq!(
                validate_helper_hello(
                    HelperResponse::Error {
                        code: "collectorUnavailable".to_owned(),
                        message: "ETW failed".to_owned(),
                    },
                    "unused",
                )
                .unwrap_err()
                .code,
                NetworkMonitorErrorCode::CollectorUnavailable
            );
        }

        #[test]
        fn pid_cache_requires_the_same_process_creation_time() {
            let mut resolver = ProcessResolver::default();
            resolver
                .cache
                .insert(42, identity_from_path(r"C:\Apps\first.exe", 100));
            assert_eq!(
                resolver.cached_identity(42, 100).unwrap().display_name,
                "first.exe"
            );
            assert!(resolver.cached_identity(42, 101).is_none());
            assert_eq!(
                unknown_process_identity().application_id,
                UNKNOWN_APPLICATION_ID
            );
        }

        #[test]
        fn proxy_classifier_keeps_direct_proxy_and_unknown_distinct() {
            let proxy = "127.0.0.1:7890".parse().unwrap();
            let direct = "203.0.113.10:443".parse().unwrap();
            let classifier = ProxyClassifier {
                endpoints: HashSet::from([proxy]),
                refreshed_at: Some(Instant::now()),
                ..ProxyClassifier::default()
            };
            assert_eq!(classifier.classify(Some(proxy)), NetworkPath::Proxy);
            assert_eq!(classifier.classify(Some(direct)), NetworkPath::Direct);

            let classifier = ProxyClassifier {
                automatic_proxy_enabled: true,
                refreshed_at: Some(Instant::now()),
                ..ProxyClassifier::default()
            };
            assert_eq!(classifier.classify(Some(direct)), NetworkPath::Unknown);
            assert_eq!(classifier.classify(None), NetworkPath::Unknown);
        }
    }
}

#[cfg(windows)]
pub use windows_helper::Client as HelperClient;

#[cfg(windows)]
pub fn try_run_from_args() -> bool {
    windows_helper::try_run_from_args()
}

#[cfg(not(windows))]
pub fn try_run_from_args() -> bool {
    false
}
