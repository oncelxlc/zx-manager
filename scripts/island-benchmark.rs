use super::{Event, IslandState, ManagerHandle};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    mem::size_of,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    },
};

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use windows_sys::Win32::System::{
    Threading::GetCurrentThread, WindowsProgramming::QueryThreadCycleTime,
};

static CALLBACKS: AtomicU64 = AtomicU64::new(0);
static CYCLES: AtomicU64 = AtomicU64::new(0);
static NANOS: AtomicU64 = AtomicU64::new(0);

pub(in crate::island) struct FrameCost(u64, Instant);

impl FrameCost {
    pub(in crate::island) fn begin() -> Option<Self> {
        let mut cycles = 0;
        (unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut cycles) } != 0)
            .then(|| Self(cycles, Instant::now()))
    }
}
impl Drop for FrameCost {
    fn drop(&mut self) {
        let nanos = self.1.elapsed().as_nanos() as u64;
        let mut cycles = 0;
        if unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut cycles) } != 0 {
            CYCLES.fetch_add(cycles - self.0, Relaxed);
            NANOS.fetch_add(nanos, Relaxed);
            CALLBACKS.fetch_add(1, Relaxed);
        }
    }
}

fn sample() -> Result<BTreeMap<u32, (u64, u64)>, String> {
    let mut parents = BTreeMap::new();
    let pid = std::process::id();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut more = Process32FirstW(snap, &mut entry);
        while more != 0 {
            let name = &entry.szExeFile[..entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len())];
            if String::from_utf16_lossy(name).eq_ignore_ascii_case("msedgewebview2.exe") {
                parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            }
            more = Process32NextW(snap, &mut entry);
        }
        CloseHandle(snap);
    }
    let mut children = BTreeSet::from([pid]);
    loop {
        let n = children.len();
        for (&child, &parent) in &parents {
            if children.contains(&parent) {
                children.insert(child);
            }
        }
        if children.len() == n {
            break;
        }
    }
    let mut result = BTreeMap::new();
    let mut root_start = 0;
    for child in std::iter::once(pid).chain(children.into_iter().filter(|&child| child != pid)) {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, child);
            if handle.is_null() {
                return Err(format!(
                    "open process {child}: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let (mut created, mut exited, mut kernel, mut user) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            let mut memory = PROCESS_MEMORY_COUNTERS::default();
            let ok = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user)
                != 0
                && GetProcessMemoryInfo(
                    handle,
                    &mut memory,
                    size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                ) != 0;
            CloseHandle(handle);
            if !ok {
                return Err(format!("sample process {child} failed"));
            }
            let ticks =
                |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
            if child == pid {
                root_start = ticks(created);
            }
            if ticks(created) < root_start {
                continue;
            }
            result.insert(
                child,
                (ticks(kernel) + ticks(user), memory.WorkingSetSize as u64),
            );
        }
    }
    Ok(result)
}

thread_local! { static DRIVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

pub(super) fn trace(event: Event, state: IslandState) {
    if !DRIVING.get() && !matches!(event, Event::PointerEnter | Event::PointerLeave) {
        eprintln!("non-workload event {event:?} while {state:?}");
    }
}

fn click(handle: &ManagerHandle, expected: IslandState) -> Result<(), String> {
    tauri::async_runtime::block_on(handle.request(move |manager| {
        let previous = manager.machine.state;
        DRIVING.set(true);
        let transition = manager.transition(Event::Click);
        DRIVING.set(false);
        transition?;
        if manager.machine.state != expected || manager.last_error.is_some() {
            return Err(format!(
                "click from {previous:?}, expected {expected:?}, got {:?}, error {:?}, drag {:?}, clickEnabled {}",
                manager.machine.state, manager.last_error, manager.drag_origin, manager.config.interaction.click_expand
            ));
        }
        Ok(())
    }))
}

fn wait_for(path: &Path, file: &str, seconds: u64) -> Result<(), String> {
    let started = Instant::now();
    while !path.join(file).is_file() {
        if started.elapsed() > Duration::from_secs(seconds) {
            return Err(format!("timed out waiting for {file}"));
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn ticks() -> Result<i64, String> {
    use windows_sys::Win32::System::Performance::QueryPerformanceCounter;
    let mut ticks = 0;
    if unsafe { QueryPerformanceCounter(&mut ticks) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(ticks)
}

fn enable_peek(handle: &ManagerHandle) -> Result<(), String> {
    tauri::async_runtime::block_on(handle.request(|manager| {
        let mut config = manager.read_config()?;
        config.edge.auto_hide = true;
        config.edge.mode = crate::island::config::EdgeMode::Peek;
        config.edge.hide_delay_ms = 60000;
        manager.save(config)?;
        Ok(())
    }))
}

fn lifecycle(handle: &ManagerHandle, path: &Path) -> Result<(), String> {
    use std::fmt::Write;
    enable_peek(handle)?;
    thread::sleep(Duration::from_millis(550));
    let mut inputs = String::from("action,ticks\n");
    for _ in 0..4 {
        for (name, event, expected) in [
            ("hide", Event::Hide, IslandState::Hidden),
            ("show", Event::Show, IslandState::Compact),
            ("peek", Event::HideTimeout, IslandState::EdgePeek),
            ("wake", Event::Click, IslandState::Compact),
        ] {
            if name == "peek" {
                tauri::async_runtime::block_on(
                    handle.request(|m| m.transition(Event::PointerLeave)),
                )?;
            }
            let start = ticks()?;
            tauri::async_runtime::block_on(handle.request(move |m| {
                m.transition(event)?;
                if m.machine.state != expected || m.last_error.is_some() {
                    return Err(format!(
                        "lifecycle {name}, state {:?}, error {:?}",
                        m.machine.state, m.last_error
                    ));
                }
                Ok(())
            }))?;
            writeln!(inputs, "{name},{start}").unwrap();
            thread::sleep(Duration::from_millis(550));
        }
    }
    std::fs::write(path.join("lifecycle-end"), ticks()?.to_string()).map_err(|e| e.to_string())?;
    std::fs::write(path.join("lifecycle-input.tmp"), inputs).map_err(|e| e.to_string())?;
    std::fs::rename(
        path.join("lifecycle-input.tmp"),
        path.join("lifecycle-input.csv"),
    )
    .map_err(|e| e.to_string())
}

fn visual(handle: &ManagerHandle, path: &Path) -> Result<(), String> {
    use std::fmt::Write;
    wait_for(path, "visual-ready", 40)?;
    thread::sleep(Duration::from_secs(2));
    let mut inputs = String::from("phase,state,ticks\n");
    for (phase, ms) in [("cycles", 550), ("reverse", 80)] {
        for _ in 0..12 {
            for (expected, label) in [
                (IslandState::Expanded, "expanded"),
                (IslandState::Compact, "compact"),
            ] {
                let ticks = ticks()?;
                click(handle, expected)?;
                writeln!(inputs, "{phase},{label},{ticks}").unwrap();
                thread::sleep(Duration::from_millis(ms));
            }
        }
    }
    thread::sleep(Duration::from_millis(550));
    std::fs::write(path.join("visual-end"), ticks()?.to_string()).map_err(|e| e.to_string())?;
    lifecycle(handle, path)?;
    for (label, expected) in [
        ("compact", IslandState::Compact),
        ("expanded", IslandState::Expanded),
        ("pinned", IslandState::Pinned),
        ("edgePeek", IslandState::EdgePeek),
    ] {
        tauri::async_runtime::block_on(handle.request(move |m| {
            match expected {
                IslandState::Expanded => m.transition(Event::Click)?,
                IslandState::Pinned => m.transition(Event::Pin)?,
                IslandState::EdgePeek => {
                    m.transition(Event::Unpin)?;
                    m.transition(Event::Click)?;
                    m.transition(Event::PointerLeave)?;
                    m.transition(Event::HideTimeout)?;
                }
                _ => {}
            }
            if m.machine.state != expected {
                return Err(format!("snapshot {label}: {:?}", m.machine.state));
            }
            Ok(())
        }))?;
        thread::sleep(Duration::from_millis(650));
        std::fs::write(path.join(format!("snap-{label}")), "").map_err(|e| e.to_string())?;
        wait_for(path, &format!("seen-{label}"), 10)?;
    }
    std::fs::write(path.join("visual-input.tmp"), inputs).map_err(|e| e.to_string())?;
    std::fs::rename(path.join("visual-input.tmp"), path.join("visual-input.csv"))
        .map_err(|e| e.to_string())?;
    wait_for(path, "visual-output.json", 20)
}

fn check_ready(handle: &ManagerHandle) -> Result<(), String> {
    tauri::async_runtime::block_on(handle.request(|m| {
        if let Some(error) = m.last_error.clone() {
            return Err(error);
        }
        m.performance()?;
        let views = m.app.webview_windows();
        if views.len() != 1 || !views.contains_key("main") {
            return Err("expected only the main Tauri webview window".into());
        }
        Ok(())
    }))
}

fn management_smoke(handle: &ManagerHandle) -> Result<(), String> {
    use crate::island::config::EdgeMode;
    let revision = tauri::async_runtime::block_on(handle.request(|m| {
        let mut draft = m.read_config()?;
        draft.appearance.background = "#304254".into();
        draft.hotkeys.show = "Ctrl+Alt+Shift+F24".into();
        draft.edge.mode = EdgeMode::Peek;
        draft.edge.auto_hide = true;
        draft.edge.hide_delay_ms = 60000;
        m.transition(Event::Hide)?;
        m.transition(Event::Show)?;
        let saved = m.save(draft)?;
        for _ in 0..4 {
            m.transition(Event::PointerLeave)?;
            m.transition(Event::HideTimeout)?;
            if m.machine.state != IslandState::EdgePeek {
                return Err("expected edge peek".into());
            }
            m.transition(Event::Click)?;
            if m.machine.state != IslandState::Compact {
                return Err("expected compact after wake".into());
            }
        }
        m.restart()?;
        Ok(saved.revision)
    }))?;
    check_ready(handle)?;
    tauri::async_runtime::block_on(handle.request(move |m| {
        let mut saved = m.read_config()?;
        if saved.revision != revision || saved.appearance.background != "#304254" {
            return Err("saved appearance did not survive restart".into());
        }
        saved.hotkeys.show.clear();
        m.save(saved)?;
        Ok(())
    }))
}

fn run(handle: &ManagerHandle, path: &Path) -> Result<(), String> {
    check_ready(handle)?;
    if std::env::var("ISLAND_BENCH_VISUAL").as_deref() == Ok("1") {
        visual(handle, path)?;
        return management_smoke(handle);
    }
    let warm_idle = std::env::var("ISLAND_BENCH_WARM_IDLE").as_deref() == Ok("1");
    thread::sleep(Duration::from_secs(if warm_idle { 20 } else { 3 }));
    let phases = if warm_idle {
        [("warm1", 0, 0, 0), ("warm2", 0, 0, 0), ("warm3", 0, 0, 0)]
    } else {
        [
            ("idle", 0, 0, 0),
            ("cycles", 100, 345, 285),
            ("reverse", 100, 35, 35),
        ]
    };
    for (phase, cycles, expanded_ms, compact_ms) in phases {
        let mut dispatch = Vec::new();
        let before = sample()?;
        let marker = json!({"phase": phase, "pids": before.keys().collect::<Vec<_>>()});
        std::fs::write(path.join("phase.json"), marker.to_string()).map_err(|e| e.to_string())?;
        let before_callbacks = CALLBACKS.load(Relaxed);
        let before_cycles = CYCLES.load(Relaxed);
        let before_nanos = NANOS.load(Relaxed);
        let start = Instant::now();
        if cycles == 0 {
            thread::sleep(Duration::from_secs(10));
        }
        for _ in 0..cycles {
            for (duration, expected) in [
                (expanded_ms, IslandState::Expanded),
                (compact_ms, IslandState::Compact),
            ] {
                let started = Instant::now();
                click(handle, expected)?;
                dispatch.push(started.elapsed().as_micros() as u64);
                thread::sleep(Duration::from_millis(duration));
            }
        }
        let elapsed = start.elapsed().as_secs_f64();
        std::fs::remove_file(path.join("phase.json")).map_err(|e| e.to_string())?;
        let after = sample()?;
        if before.keys().ne(after.keys()) {
            return Err(format!("process tree changed in {phase}"));
        }
        let ticks = after
            .iter()
            .map(|(id, (ticks, _))| ticks - before[id].0)
            .sum::<u64>();
        let host_ticks = after[&std::process::id()].0 - before[&std::process::id()].0;
        let ws = after.values().map(|(_, ws)| *ws).sum::<u64>();
        dispatch.sort_unstable();
        let quantile = |n: usize| {
            if dispatch.is_empty() {
                None
            } else {
                Some(dispatch[(dispatch.len() * n).div_ceil(100) - 1])
            }
        };
        let callbacks = CALLBACKS.load(Relaxed) - before_callbacks;
        let average_cycles = (CYCLES.load(Relaxed) - before_cycles).checked_div(callbacks);
        let average_ns = (NANOS.load(Relaxed) - before_nanos).checked_div(callbacks);
        let renderer = tauri::async_runtime::block_on(handle.request(|m| m.performance()))?;
        let data = json!({
            "phase": phase,
            "renderer_stats": renderer,
            "animation_callbacks": callbacks,
            "mean_callback_cpu_cycles": average_cycles,
            "mean_callback_wall_ns": average_ns,
            "elapsed_s": elapsed,
            "process_count": after.len(),
            "one_core_cpu_pct": ticks as f64 / 100000.0 / elapsed,
            "host_cpu_pct": host_ticks as f64 / 100000.0 / elapsed,
            "webview_cpu_pct": (ticks - host_ticks) as f64 / 100000.0 / elapsed,
            "working_set_mib": ws as f64 / 1048576.0,
            "dispatch_us": [quantile(50),quantile(95),quantile(99)]
        });
        std::fs::write(path.join(format!("{phase}.json")), data.to_string())
            .map_err(|e| e.to_string())?;
    }
    management_smoke(handle)
}

pub(in crate::island) fn start(handle: ManagerHandle) {
    thread::spawn(move || {
        let path = PathBuf::from(std::env::var_os("ISLAND_BENCH_DIR").unwrap());
        if let Err(e) = run(&handle, &path) {
            let _ = std::fs::write(path.join("error.txt"), e);
        }
        let _ = handle.post(|m| m.exit());
    });
}
