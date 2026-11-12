use serde_json::{json, Value};
use std::{
    error::Error as StdError,
    fs,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    slice, thread,
    time::{Duration, Instant},
};
use windows::{
    core::{Interface, Result as WinResult},
    Win32::{
        Foundation::{E_FAIL, HMODULE},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_UNKNOWN,
            Direct3D11::*,
            Dxgi::{Common::*, *},
        },
    },
};
use windows_sys::{
    core::{w, BOOL},
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::Gdi::{
            EnumDisplaySettingsW, GetStockObject, UpdateWindow, DEVMODEW, ENUM_CURRENT_SETTINGS,
            LTGRAY_BRUSH,
        },
        System::{LibraryLoader::GetModuleHandleW, Performance::QueryPerformanceFrequency},
        UI::{
            HiDpi::{
                GetDpiForWindow, SetThreadDpiAwarenessContext,
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            },
            WindowsAndMessaging::*,
        },
    },
};

type Result<T> = std::result::Result<T, Box<dyn StdError>>;

fn capture_indicator(bgra: &[u8]) -> bool {
    // Include the antialiased edge where yellow blends into the gray backdrop.
    bgra[2] >= bgra[1] && bgra[1] > bgra[0] && bgra[2].saturating_sub(bgra[0]) >= 6
}

struct Search {
    pid: u32,
    count: usize,
    visible: HWND,
}
unsafe extern "system" fn find(hwnd: HWND, ptr: LPARAM) -> BOOL {
    let search = &mut *(ptr as *mut Search);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == search.pid {
        let mut name = [0u16; 64];
        let len = GetClassNameW(hwnd, name.as_mut_ptr(), name.len() as i32);
        if String::from_utf16_lossy(&name[..len.max(0) as usize]) == "ZXOnce.NativeIsland.v1" {
            search.count += 1;
            if IsWindowVisible(hwnd) != 0 {
                search.visible = hwnd;
            }
        }
    }
    1
}

struct Backdrop(HWND);
impl Drop for Backdrop {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

unsafe fn backdrop(island: HWND, config_dir: &Path) -> Result<(Backdrop, RECT)> {
    let mut r = RECT::default();
    if GetWindowRect(island, &mut r) == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let scale = f64::from(GetDpiForWindow(island)) / 96.0;
    let px = |v: f64| (v * scale).round() as i32;
    let config: Value = serde_json::from_slice(&fs::read(config_dir.join("island.json"))?)?;
    let size = |key: &str| {
        config["appearance"][key]
            .as_f64()
            .map(px)
            .ok_or("缺少采样尺寸")
    };
    let (w, h) = (size("expandedWidth")?, size("expandedHeight")?);
    // The compact bar widens when Windows reports a battery or a media session.
    if !(size("compactWidth")? - 1..=w + 1).contains(&(r.right - r.left))
        || (r.bottom - r.top - size("compactHeight")?).abs() > 1
    {
        return Err("帧采样窗口与配置的收起尺寸不符".into());
    }
    let x = (r.left + r.right - w) / 2;
    let y = r.top;
    let cls = WNDCLASSW {
        lpfnWndProc: Some(DefWindowProcW),
        hInstance: GetModuleHandleW(null()),
        lpszClassName: w!("ZXOnce.BenchmarkBackdrop"),
        hbrBackground: GetStockObject(LTGRAY_BRUSH),
        ..Default::default()
    };
    if RegisterClassW(&cls) == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let bg = Backdrop(CreateWindowExW(
        WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        cls.lpszClassName,
        w!("zx-once 帧测量背景"),
        WS_DISABLED | WS_POPUP,
        x,
        y,
        w,
        h,
        null_mut(),
        null_mut(),
        cls.hInstance,
        null(),
    ));
    if bg.0.is_null()
        || SetWindowPos(bg.0, island, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW) == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    UpdateWindow(bg.0);
    Ok((
        bg,
        RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        },
    ))
}

struct Capture {
    dup: IDXGIOutputDuplication,
    dc: ID3D11DeviceContext,
    staging: ID3D11Texture2D,
    region: D3D11_BOX,
    last: Vec<u8>,
    hz: u32,
    excluded_top_rows: usize,
}

#[derive(Clone, Copy)]
struct Sample {
    ticks: i64,
    dark: usize,
    covered: usize,
    accumulated: u32,
}

impl Capture {
    unsafe fn new(r: RECT) -> Result<Self> {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        for a in 0.. {
            let adapter = match factory.EnumAdapters1(a) {
                Ok(adapter) => adapter,
                Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(e) => return Err(e.into()),
            };
            for i in 0.. {
                let output = match adapter.EnumOutputs(i) {
                    Ok(output) => output,
                    Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
                    Err(e) => return Err(e.into()),
                };
                let desc = output.GetDesc()?;
                let d = desc.DesktopCoordinates;
                if !desc.AttachedToDesktop.as_bool()
                    || r.left < d.left
                    || r.right > d.right
                    || r.top < d.top
                    || r.bottom > d.bottom
                {
                    continue;
                }
                if desc.Rotation != DXGI_MODE_ROTATION_IDENTITY {
                    return Err("请在未旋转的测试显示器运行帧采样".into());
                }
                let raw: IDXGIAdapter = adapter.cast()?;
                let mut d3d = None;
                D3D11CreateDevice(
                    &raw,
                    D3D_DRIVER_TYPE_UNKNOWN,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut d3d),
                    None,
                    None,
                )?;
                let d3d = d3d.ok_or("D3D11 device missing")?;
                let dc = d3d.GetImmediateContext()?;
                let dup = output.cast::<IDXGIOutput1>()?.DuplicateOutput(&d3d)?;
                let (w, h) = ((r.right - r.left) as u32, (r.bottom - r.top) as u32);
                let texture = D3D11_TEXTURE2D_DESC {
                    Width: w,
                    Height: h,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_STAGING,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    ..Default::default()
                };
                let mut staging = None;
                d3d.CreateTexture2D(&texture, None, Some(&mut staging))?;
                let region = D3D11_BOX {
                    left: (r.left - d.left) as u32,
                    top: (r.top - d.top) as u32,
                    front: 0,
                    right: (r.right - d.left) as u32,
                    bottom: (r.bottom - d.top) as u32,
                    back: 1,
                };
                let mut mode = DEVMODEW {
                    dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                    ..Default::default()
                };
                if EnumDisplaySettingsW(desc.DeviceName.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode)
                    == 0
                    || mode.dmDisplayFrequency <= 1
                {
                    return Err("无法读取显示器刷新率".into());
                }
                return Ok(Self {
                    dup,
                    dc,
                    staging: staging.ok_or("D3D11 staging texture missing")?,
                    region,
                    last: vec![0; (w * h * 4) as usize],
                    hz: mode.dmDisplayFrequency,
                    excluded_top_rows: 0,
                });
            }
        }
        Err("测试窗口超出显示器边界".into())
    }

    unsafe fn sample(&mut self) -> WinResult<Option<Sample>> {
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource = None;
        match self.dup.AcquireNextFrame(80, &mut info, &mut resource) {
            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => return Ok(None),
            result => result?,
        }
        let result = (|| {
            if info.LastPresentTime == 0 {
                return Ok(None);
            }
            let image: ID3D11Texture2D = resource
                .as_ref()
                .ok_or_else(|| windows::core::Error::from_hresult(E_FAIL))?
                .cast()?;
            // Only the gray-backed island rectangle ever leaves GPU memory.
            self.dc
                .CopySubresourceRegion(&self.staging, 0, 0, 0, 0, &image, 0, Some(&self.region));
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.dc
                .Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
            let width = (self.region.right - self.region.left) as usize;
            let mut changed = false;
            let mut dark = 0;
            let mut covered = 0;
            let mut border = 0;
            for (y, prev) in self.last.chunks_exact_mut(width * 4).enumerate() {
                let row = slice::from_raw_parts(
                    mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize),
                    prev.len(),
                );
                // Windows can overlay its yellow capture indicator along the display edge.
                // Both ends of this strip are outside the rounded island in our fixture.
                if y == border
                    && y < 6
                    && capture_indicator(row)
                    && capture_indicator(&row[row.len() - 4..])
                {
                    border += 1;
                    for pixel in prev.as_chunks_mut::<4>().0 {
                        *pixel = [192, 192, 192, 255];
                    }
                    continue;
                }
                if prev != row {
                    changed = true;
                    prev.copy_from_slice(row);
                }
                if y % 4 == 0 {
                    for &blue in row.iter().step_by(16) {
                        dark += usize::from(blue < 160);
                        covered += usize::from(blue.abs_diff(192) >= 6);
                    }
                }
            }
            self.excluded_top_rows = self.excluded_top_rows.max(border);
            self.dc.Unmap(&self.staging, 0);
            Ok(changed.then_some(Sample {
                ticks: info.LastPresentTime,
                dark,
                covered,
                accumulated: info.AccumulatedFrames,
            }))
        })();
        self.dup.ReleaseFrame()?;
        result
    }
}

fn percentiles(mut values: Vec<f64>) -> Value {
    values.sort_by(f64::total_cmp);
    json!([50usize, 95, 99]
        .map(|p| (!values.is_empty()).then(|| values[(values.len() * p).div_ceil(100) - 1])))
}

fn lifecycle(csv: &str, frames: &[Sample], frequency: i64, end: i64) -> Result<Value> {
    let actions = ["hide", "show", "peek", "wake"];
    let mut markers = Vec::new();
    for (i, line) in csv.lines().skip(1).enumerate() {
        let (action, ticks) = line
            .trim()
            .split_once(',')
            .ok_or("invalid lifecycle marker")?;
        if action != actions[i % 4] {
            return Err("unexpected lifecycle action".into());
        }
        markers.push((action, ticks.parse::<i64>()?));
    }
    if markers.len() != 16 || end <= markers[15].1 || !markers.windows(2).all(|p| p[0].1 < p[1].1) {
        return Err("invalid lifecycle sequence".into());
    }
    let mut result = json!({});
    for action in actions {
        let (mut first, mut last) = (Vec::new(), Vec::new());
        let mut missing = 0;
        let mut matching = 0;
        for (i, &(label, start)) in markers.iter().enumerate() {
            if label != action {
                continue;
            }
            let bound = markers.get(i + 1).map_or(end, |m| m.1);
            let lo = frames.partition_point(|f| f.ticks < start);
            let hi = frames.partition_point(|f| f.ticks < bound);
            let changed = &frames[lo..hi];
            if let (Some(a), Some(b)) = (changed.first(), changed.last()) {
                let elapsed = |t: i64| (t - start) as f64 * 1000.0 / frequency as f64;
                first.push(elapsed(a.ticks));
                last.push(elapsed(b.ticks));
            } else {
                missing += 1;
            }
            if hi.checked_sub(1).is_some_and(|n| match action {
                "hide" => frames[n].covered == 0,
                "peek" => frames[n].covered > 0 && frames[n].dark == 0,
                _ => frames[n].dark > 0,
            }) {
                matching += 1;
            }
        }
        result[action] = json!({"firstChangeMs":percentiles(first),"lastChangeMs":percentiles(last),
            "missingChanges":missing,"matchingStableSamples":matching,"trials":4});
    }
    Ok(result)
}

fn stable_image(buffer: &[u8], width: usize) -> Value {
    let background = &[192, 192, 192];
    let mut colors = std::collections::BTreeMap::new();
    let (mut left, mut top, mut right, mut bottom) = (usize::MAX, usize::MAX, 0, 0);
    let mut area = 0;
    let mut bright_ink = 0;
    let mut pinned_fill = 0;
    for (i, pixel) in buffer.as_chunks::<4>().0.iter().enumerate() {
        if pixel
            .iter()
            .zip(background)
            .all(|(&c, &b)| c.abs_diff(b) < 6)
        {
            continue;
        }
        let (x, y) = (i % width, i / width);
        left = left.min(x);
        top = top.min(y);
        right = right.max(x + 1);
        bottom = bottom.max(y + 1);
        *colors
            .entry([pixel[2], pixel[1], pixel[0]])
            .or_insert(0usize) += 1;
        area += 1;
        if pixel[..3].iter().all(|&c| c > 224) {
            bright_ink += 1;
        }
        if pixel[1].saturating_sub(pixel[2]) > 20 && pixel[1].saturating_sub(pixel[0]) > 8 {
            pinned_fill += 1;
        }
    }
    let bbox = (area > 0).then_some([left, top, right, bottom]);
    let dominant = colors
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color);
    json!({"boundsPx":bbox,"pixels":area,"dominantRgb":dominant,"brightInkPx":bright_ink,"pinnedFillPx":pinned_fill,
        "backdropRgb":[background[2],background[1],background[0]]})
}

fn analyze(csv: &str, frames: &[Sample], frequency: i64, hz: u32) -> Result<Value> {
    let mut markers = Vec::new();
    for line in csv.lines().skip(1) {
        let mut parts = line.trim().split(',');
        markers.push((
            parts.next().ok_or("missing phase")?,
            parts.next().ok_or("missing state")?,
            parts.next().ok_or("missing ticks")?.parse::<i64>()?,
        ));
    }
    if markers.len() != 48 || !markers.windows(2).all(|p| p[0].2 < p[1].2) {
        return Err("unexpected input markers".into());
    }
    for (i, &(phase, state, _)) in markers.iter().enumerate() {
        if phase != if i < 24 { "cycles" } else { "reverse" }
            || state != if i % 2 == 0 { "expanded" } else { "compact" }
        {
            return Err("unexpected benchmark workload".into());
        }
    }
    let ms = |ticks: i64| ticks as f64 * 1000.0 / frequency as f64;
    let mut result = json!({"refreshHz":hz,"qpcFrequency":frequency,
        "idleExtraChanges":frames.iter().filter(|f| f.ticks<markers[0].2).count().saturating_sub(1)});
    for phase in ["cycles", "reverse"] {
        let mut intervals = Vec::new();
        let mut first = Vec::new();
        let mut last = Vec::new();
        let mut changes = 0;
        let mut coalesced = 0;
        let mut no_response = 0;
        for (i, &(group, state, start)) in markers.iter().enumerate() {
            if group != phase {
                continue;
            }
            let end = markers.get(i + 1).map_or(i64::MAX, |m| m.2);
            let lo = frames.partition_point(|f| f.ticks < start);
            let hi = frames.partition_point(|f| f.ticks < end);
            let active = &frames[lo..hi];
            changes += active.len();
            coalesced += active.iter().filter(|f| f.accumulated > 1).count();
            intervals.extend(active.windows(2).map(|p| ms(p[1].ticks - p[0].ticks)));
            if let Some(frame) = active.last() {
                last.push(ms(frame.ticks - start));
            }
            // During a reversal pixels may still move in the old direction after dispatch.
            // Count the first compositor sample whose dark footprint moves in the requested direction.
            let mut previous = lo.checked_sub(1).map(|n| frames[n].dark);
            let response = active.iter().find(|f| {
                let old = previous.replace(f.dark);
                old.is_some_and(|n| {
                    if state == "expanded" {
                        f.dark > n + 2
                    } else {
                        f.dark + 2 < n
                    }
                })
            });
            if let Some(frame) = response {
                first.push(ms(frame.ticks - start));
            } else {
                no_response += 1;
            }
        }
        let gaps = intervals
            .iter()
            .filter(|t| **t > 1500.0 / f64::from(hz))
            .count();
        let count = intervals.len();
        result[phase] = json!({"changedFrames":changes,"intervalCount":count,"coalescedSamples":coalesced,
            "intervalMs":percentiles(intervals),"gapsOver1_5Periods":gaps,
            "requestedDirectionMs":percentiles(first),"missingDirectionResponses":no_response,
            "lastChangeMs":percentiles(last)});
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_capture_border_blended_into_the_backdrop() {
        assert!(capture_indicator(&[48, 214, 239, 255]));
        assert!(capture_indicator(&[182, 194, 196, 255]));
        assert!(!capture_indicator(&[192, 192, 192, 255]));
        assert!(!capture_indicator(&[44, 42, 39, 255]));
    }
    #[test]
    fn lifecycle_compares_hidden_peek_and_visible_footprints() {
        let mut csv = String::from("action,ticks\n");
        let mut frames = Vec::new();
        for i in 0..16 {
            let (name, dark, covered) = [
                ("hide", 0, 0),
                ("show", 10, 12),
                ("peek", 0, 4),
                ("wake", 10, 12),
            ][i % 4];
            let start = i as i64 * 500;
            csv.push_str(&format!("{name},{start}\n"));
            if i != 4 {
                frames.push(Sample {
                    ticks: start + 17,
                    dark,
                    covered,
                    accumulated: 1,
                });
            }
        }
        let result = lifecycle(&csv, &frames, 1000, 8000).unwrap();
        assert_eq!(result["peek"]["matchingStableSamples"], 4);
        assert_eq!(result["hide"]["matchingStableSamples"], 3);
        assert_eq!(result["hide"]["missingChanges"], 1);
        assert_eq!(result["wake"]["firstChangeMs"], json!([17.0, 17.0, 17.0]));
    }
    #[test]
    fn intervals_exclude_idle_holds_and_use_requested_direction() {
        let mut csv = String::from("phase,state,ticks\n");
        let mut frames = vec![Sample {
            ticks: 1,
            dark: 20,
            covered: 20,
            accumulated: 1,
        }];
        for i in 0..48 {
            let t = 1000 + i * 600;
            let expanded = i % 2 == 0;
            csv.push_str(&format!(
                "{},{},{}\n",
                if i < 24 { "cycles" } else { "reverse" },
                if expanded { "expanded" } else { "compact" },
                t
            ));
            for (offset, dark) in [
                (6, if expanded { 20 } else { 48 }),
                (20, 32),
                (36, if expanded { 48 } else { 20 }),
            ] {
                frames.push(Sample {
                    ticks: t + offset,
                    dark,
                    covered: dark,
                    accumulated: 1,
                });
            }
        }
        let result = analyze(&csv, &frames, 1000, 60).unwrap();
        for phase in ["cycles", "reverse"] {
            assert_eq!(result[phase]["intervalMs"], json!([14.0, 16.0, 16.0]));
            assert_eq!(
                result[phase]["requestedDirectionMs"],
                json!([20.0, 20.0, 20.0])
            );
            assert_eq!(result[phase]["missingDirectionResponses"], 0);
        }
    }
}

fn collect(pid: u32, dir: &Path) -> Result<()> {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let start = Instant::now();
    let island = loop {
        let mut search = Search {
            pid,
            count: 0,
            visible: null_mut(),
        };
        if unsafe { EnumWindows(Some(find), (&raw mut search) as isize) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if search.count > 1 {
            return Err("指定实例存在多个原生灵动岛窗口".into());
        }
        if !search.visible.is_null() {
            break search.visible;
        }
        if start.elapsed() > Duration::from_secs(35) {
            return Err("未发现指定实例可见的原生灵动岛窗口".into());
        }
        thread::sleep(Duration::from_millis(100));
    };
    let (_backdrop, roi) = unsafe { backdrop(island, dir) }?;
    let mut capture = unsafe { Capture::new(roi) }?;
    let mut ticks = 0;
    if unsafe { QueryPerformanceFrequency(&mut ticks) } == 0 {
        return Err("QPC unavailable".into());
    }
    let mut frames = Vec::new();
    let input = dir.join("visual-input.csv");
    let first = Instant::now();
    let mut ready = false;
    let mut stable = json!({});
    let mut pending = ["compact", "expanded", "pinned", "edgePeek"]
        .into_iter()
        .peekable();
    while !input.exists() {
        if let Some(sample) = unsafe { capture.sample() }? {
            frames.push(sample);
            if !ready {
                fs::write(dir.join("visual-ready"), b"ready")?;
                ready = true;
            }
        }
        if let Some(&state) = pending.peek() {
            if dir.join(format!("snap-{state}")).exists() {
                stable[state] = stable_image(&capture.last, (roi.right - roi.left) as usize);
                fs::write(dir.join(format!("seen-{state}")), b"ok")?;
                pending.next();
            }
        }
        if first.elapsed() > Duration::from_secs(55) {
            return Err("等待帧采样动作超时".into());
        }
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
    let end = dir.join("visual-end");
    let motion_count = if end.exists() {
        let until: i64 = fs::read_to_string(end)?.trim().parse()?;
        frames.partition_point(|f| f.ticks <= until)
    } else {
        frames.len()
    };
    let mut summary = analyze(
        &fs::read_to_string(input)?,
        &frames[..motion_count],
        ticks,
        capture.hz,
    )?;
    let sequence = dir.join("lifecycle-input.csv");
    if sequence.exists() {
        let until = fs::read_to_string(dir.join("lifecycle-end"))?
            .trim()
            .parse()?;
        summary["lifecycle"] = lifecycle(&fs::read_to_string(sequence)?, &frames, ticks, until)?;
    }
    let mut visible = true;
    for state in ["compact", "expanded", "pinned", "edgePeek"] {
        if let Some(snapshot) = stable.get(state) {
            visible &= snapshot["brightInkPx"].as_u64().unwrap_or(0) > 0;
            if state == "pinned" {
                visible &= snapshot["boundsPx"] == stable["expanded"]["boundsPx"];
            }
        }
    }
    summary["excludedTopRows"] = json!(capture.excluded_top_rows);
    summary["visibleContentChecksPassed"] = json!(visible);
    summary["stable"] = stable;
    fs::write(
        dir.join("visual-output.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{summary}");
    if !visible {
        return Err("岛屿内容未显示或固定态尺寸异常".into());
    }
    Ok(())
}

pub(super) fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let pid = args
        .next()
        .ok_or("usage: island_frame_probe <benchmark PID> <benchmark directory>")?
        .parse()?;
    let dir = PathBuf::from(args.next().ok_or("missing benchmark directory")?);
    if !dir.is_dir()
        || ["visual-ready", "visual-input.csv", "visual-output.json"]
            .iter()
            .any(|s| dir.join(s).exists())
    {
        return Err("要求新的基准运行目录".into());
    }
    collect(pid, &dir)
}
