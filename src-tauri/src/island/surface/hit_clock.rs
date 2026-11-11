use super::{error, Result};
use std::{
    cell::Cell,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr::null,
};
use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::{Performance::QueryPerformanceCounter, Threading::*},
    UI::WindowsAndMessaging::{MsgWaitForMultipleObjectsEx, MWMO_INPUTAVAILABLE, QS_ALLINPUT},
};

// DirectComposition animates pixels, but USER32 only consults a window region across processes.
// Wake at compositor boundaries only while that hit region is changing.
pub(super) struct HitClock {
    timer: OwnedHandle,
    cadence: Cell<Option<(i64, i64, i64)>>,
}

impl HitClock {
    pub fn new() -> Result<Self> {
        let open = |flags| unsafe {
            CreateWaitableTimerExW(
                null(),
                null(),
                flags,
                SYNCHRONIZATION_SYNCHRONIZE | TIMER_MODIFY_STATE,
            )
        };
        let mut handle = open(CREATE_WAITABLE_TIMER_HIGH_RESOLUTION);
        if handle.is_null() {
            handle = open(0);
        }
        if handle.is_null() {
            return Err(error("创建命中区域等待定时器失败"));
        }
        Ok(Self {
            timer: unsafe { OwnedHandle::from_raw_handle(handle) },
            cadence: Cell::new(None),
        })
    }
    fn handle(&self) -> HANDLE {
        self.timer.as_raw_handle()
    }
    pub fn start(&self, cadence: (i64, i64, i64)) {
        self.cadence.set(Some(cadence));
    }
    pub fn active(&self) -> bool {
        self.cadence.get().is_some()
    }
    pub fn stop(&self) {
        if self.cadence.take().is_some() {
            unsafe {
                CancelWaitableTimer(self.handle());
            }
        }
    }
    pub fn next(&self) -> Result<()> {
        let Some((phase, period, freq)) = self.cadence.get() else {
            return Ok(());
        };
        let mut now = 0;
        unsafe {
            QueryPerformanceCounter(&mut now);
        }
        let ticks = period - (now - phase).rem_euclid(period);
        let relative = -((ticks * 10_000_000 / freq).max(1));
        if unsafe { SetWaitableTimer(self.handle(), &relative, 0, None, null(), 0) } == 0 {
            return Err(error("调度原生命中区域更新失败"));
        }
        Ok(())
    }
    pub fn wait(&self) -> Result<bool> {
        match unsafe {
            MsgWaitForMultipleObjectsEx(
                1,
                &self.handle(),
                INFINITE,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            )
        } {
            WAIT_OBJECT_0 => Ok(true),
            1 => Ok(false),
            _ => Err(error("等待原生窗口消息失败")),
        }
    }
}
