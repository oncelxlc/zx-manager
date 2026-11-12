use windows::{
    ApplicationModel::{StartupTask, StartupTaskState},
    Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
};
use windows_core::h;

/// Only an explicit settings change supplies a request. Startup reads the user's Windows choice.
pub(super) fn task(request: Option<bool>) -> Result<bool, String> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(task_error)?;
    let result = configure_task(request);
    unsafe { RoUninitialize() };
    result
}

fn configure_task(request: Option<bool>) -> Result<bool, String> {
    let task = StartupTask::GetAsync(h!("ZxOnceStartup"))
        .and_then(|op| op.get())
        .map_err(task_error)?;
    let mut state = task.State().map_err(task_error)?;
    let enabled = |value| {
        matches!(
            value,
            StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
        )
    };
    let Some(desired) = request else {
        return Ok(enabled(state));
    };
    if desired == enabled(state) {
        return Ok(desired);
    }
    match state {
        StartupTaskState::DisabledByUser => {
            return Err("请在 Windows「设置 → 应用 → 启动」中允许 zx-once 登录启动".into());
        }
        StartupTaskState::DisabledByPolicy | StartupTaskState::EnabledByPolicy => {
            return Err("此登录启动项受 Windows 策略管理".into());
        }
        _ => {}
    }
    if desired {
        state = task
            .RequestEnableAsync()
            .and_then(|op| op.get())
            .map_err(task_error)?;
    } else {
        task.Disable().map_err(task_error)?;
        state = task.State().map_err(task_error)?;
    }
    if enabled(state) != desired {
        return Err("Windows 未应用此更改，请检查「设置 → 应用 → 启动」".into());
    }
    Ok(desired)
}

fn task_error(error: windows_core::Error) -> String {
    format!("无法访问 Windows 登录启动项（{}）", error.code())
}
