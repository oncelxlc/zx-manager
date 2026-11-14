#Requires -Version 7.4
[CmdletBinding()]
param([string]$VisualProbe, [switch]$WarmIdle)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $IsWindows) { throw 'The island benchmark needs a Windows desktop.' }
if (-not (Test-Path "$root/dist/index.html")) { throw 'Run pnpm build first.' }
if ($WarmIdle -and $VisualProbe) { throw 'Idle and visual scenarios run separately.' }
if ($VisualProbe) { $VisualProbe = (Resolve-Path -LiteralPath $VisualProbe).Path }
$tag = [Guid]::NewGuid().ToString('N').Substring(0, 12)
$stage = Join-Path $root "src-tauri/target/island-bench-$tag"
$project = Join-Path $stage 'src-tauri'
$binary = "zx-island-measure-$tag"
New-Item -ItemType Directory -Path $project | Out-Null
# Keep the instrumented crate in its own copy of the locked application workspace.
Copy-Item -LiteralPath "$root/Cargo.toml","$root/Cargo.lock","$root/LICENSE" -Destination $stage
foreach ($name in 'src','icons','capabilities','permissions','Cargo.toml','build.rs','tauri.conf.json') {
    Copy-Item -LiteralPath "$root/src-tauri/$name" -Destination $project -Recurse
}
function Replace-Once($file, $before, $after) {
    $source = Get-Content -LiteralPath $file -Raw
    if (([regex]::Matches($source, [regex]::Escape($before))).Count -ne 1) {
        throw "Benchmark fixture no longer matches $file"
    }
    $source.Replace($before, $after) | Set-Content -LiteralPath $file -Encoding utf8NoBOM
}
# Redirect known OS side effects in the copy only. No change to the user's config or Run value.
Replace-Once "$project/src/island/mod.rs" 'app.path().app_config_dir()?' 'std::path::PathBuf::from(std::env::var_os("ISLAND_BENCH_DIR").unwrap())'
Replace-Once "$project/src/island/mod.rs" 'app.manage(handle);' 'manager::benchmark::start(handle.clone()); app.manage(handle);'
Replace-Once "$project/src/island/native.rs" 'pub fn set_autostart(enabled: bool) -> Result<(), String> {' 'pub fn set_autostart(enabled: bool) -> Result<(), String> { if std::env::var_os("ISLAND_BENCH_DIR").is_some() {return Ok(());}'
Replace-Once "$project/src/island/manager.rs" 'pub fn transition(&mut self, event: Event) -> Result<()> {' 'pub fn transition(&mut self, event: Event) -> Result<()> { benchmark::trace(event, self.machine.state);'
$maskTick = 'if ui.clock.wait()? {'
$frameProbe = 'let _frame_cost = crate::island::manager::benchmark::FrameCost::begin();'
Replace-Once "$project/src/island/surface.rs" $maskTick "$maskTick $frameProbe"
Add-Content "$project/src/island/manager.rs" 'pub(super) mod benchmark;'
Copy-Item "$PSScriptRoot/island-benchmark.rs" "$project/src/island/manager/benchmark.rs"
Replace-Once "$project/Cargo.toml" '"Win32_Foundation",' '"Win32_Foundation", "Win32_System_ProcessStatus", "Win32_System_WindowsProgramming",'
Add-Content "$project/Cargo.toml" "`n[[bin]]`nname = `"$binary`"`npath = `"src/main.rs`""

$settings = Get-Content "$project/tauri.conf.json" -Raw | ConvertFrom-Json
$settings.identifier = "com.oncel.island-bench-$tag"
$settings.build.devUrl = 'http://127.0.0.1:16421'
$settings.app.windows[0] | Add-Member NoteProperty visible $false -Force
$settings | ConvertTo-Json -Depth 10 | Set-Content "$project/tauri.conf.json" -Encoding utf8NoBOM
& cargo build --locked --manifest-path "$project/Cargo.toml" --target-dir "$root/src-tauri/target" --bin $binary
if ($LASTEXITCODE -ne 0) { throw 'Benchmark build failed.' }
$exe = "$root/src-tauri/target/debug/$binary.exe"

$config = Get-Content "$root/src/features/island/defaults.json" -Raw | ConvertFrom-Json
# Keep the renderer migration baseline independent of installed messaging clients.
$config.notifications.enabled = $false
$config.placement.offsetX = 50
# Keep the edge strip inside the same capture rectangle as the expanded island.
$config.placement.offsetY = 0.0
$config.edge.autoHide = $false
$config.edge.hideOnFullscreen = $false
$config.interaction.hoverExpand = $false

function Read-Phase($directory) {
    $body = Get-Content -LiteralPath "$directory/phase.json" -Raw -ErrorAction SilentlyContinue
    if ($body) { $body | ConvertFrom-Json }
}

function Measure-Run {
    $dir = Join-Path $stage 'native'
    New-Item -ItemType Directory -Path $dir | Out-Null
    $config | ConvertTo-Json -Depth 10 | Set-Content "$dir/island.json" -Encoding utf8NoBOM
    $app = Start-Process -FilePath $exe -WindowStyle Hidden -PassThru -WorkingDirectory $root -Environment @{
        ISLAND_BENCH_WARM_IDLE = [int][bool]$WarmIdle; ISLAND_BENCH_VISUAL = [int][bool]$VisualProbe; ISLAND_BENCH_DIR = $dir; WEBVIEW2_USER_DATA_FOLDER = "$dir/webview2"
    } -RedirectStandardOutput "$dir/app.out" -RedirectStandardError "$dir/app.err"
    $probe = $null
    $samples = @{}
    $canMeasureGpu = $true
    $clock = [Diagnostics.Stopwatch]::StartNew()
    try {
        if ($VisualProbe) {
            $probe = Start-Process -FilePath $VisualProbe -ArgumentList $app.Id,('"{0}"' -f $dir) -PassThru -WindowStyle Hidden -RedirectStandardError "$dir/probe.err" -RedirectStandardOutput "$dir/probe.out"
        }
        while (-not $app.WaitForExit(100)) {
            if ($probe) {
                if ($probe.HasExited -and $probe.ExitCode -ne 0) { throw (Get-Content "$dir/probe.err" -Raw) }
                if ($clock.Elapsed.TotalSeconds -gt 100) { throw 'Visual measurement timed out.' }
                continue
            }
            if ($clock.Elapsed.TotalSeconds -gt 180) { throw 'Native benchmark timed out.' }
            $first = Read-Phase $dir
            if (-not $first -or -not $canMeasureGpu) { continue }
            try {
                $counter = Get-Counter '\GPU Engine(*)\Utilization Percentage' -SampleInterval 1 -MaxSamples 2 -ErrorAction SilentlyContinue
            } catch {
                Write-Warning "GPU counter unavailable: $_"
                $canMeasureGpu = $false
                continue
            }
            if (@($counter).Count -lt 2) {
                Write-Warning 'GPU counter returned fewer than two samples.'
                $canMeasureGpu = $false
                continue
            }
            $last = Read-Phase $dir
            if ($last.phase -ne $first.phase) { continue }
            # Sum this process tree's contributions to each engine and keep the busiest engine.
            $engines = @{}
            foreach ($entry in $counter[-1].CounterSamples) {
                if ($entry.InstanceName -match '^pid_(\d+)_(.+)$' -and [int]$Matches[1] -in $first.pids -and $entry.Status -in 0,1) {
                    $engines[$Matches[2]] += $entry.CookedValue
                }
            }
            if ($engines.Count) {
                $samples[$first.phase] = @($samples[$first.phase] | Where-Object { $null -ne $_ }) + ($engines.Values | Measure-Object -Maximum).Maximum
            }
        }
        if (Test-Path "$dir/error.txt") { throw (Get-Content "$dir/error.txt" -Raw) }
        if ($VisualProbe) {
            if (-not $probe.WaitForExit(5000) -or $probe.ExitCode -ne 0 -or $app.ExitCode -ne 0) { throw "Visual probe failed. See $dir" }
            Get-Content "$dir/visual-output.json" -Raw
            return
        }
        $phases = if ($WarmIdle) { @('warm1', 'warm2', 'warm3') } else { @('idle','cycles','reverse') }
        $lastPhase = $phases[-1]
        if ($app.ExitCode -ne 0 -or -not (Test-Path "$dir/$lastPhase.json")) { throw "Incomplete run. See $dir/app.err" }
        foreach ($phase in $phases) {
            $row = Get-Content "$dir/$phase.json" -Raw | ConvertFrom-Json
            $gpu = $samples[$phase] | Measure-Object -Average
            $row | Add-Member NoteProperty gpu_busiest_engine_pct $gpu.Average
            $row | Add-Member NoteProperty gpu_samples $gpu.Count
            $row | ConvertTo-Json -Depth 8 | Set-Content "$dir/$phase.json" -Encoding utf8NoBOM
            $row | ConvertTo-Json -Depth 8 -Compress
        }
    } finally {
        if ($probe -and -not $probe.HasExited) { $probe.Kill($true); $probe.WaitForExit() }
        if (-not $app.HasExited) { $app.Kill($true); $app.WaitForExit() }
    }
}

$server = Start-Process -FilePath (Get-Command node.exe).Source -WorkingDirectory $root -WindowStyle Hidden -PassThru -ArgumentList @(
    'node_modules/vite/bin/vite.js','preview','--host','127.0.0.1','--port','16421','--strictPort'
) -RedirectStandardError "$stage/server.err" -RedirectStandardOutput "$stage/server.out"
try {
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        if ($server.HasExited) { throw (Get-Content "$stage/server.err" -Raw) }
        if (Select-String -LiteralPath "$stage/server.out" -SimpleMatch '127.0.0.1:16421' -Quiet) { break }
        Start-Sleep -Milliseconds 100
    }
    if ($attempt -eq 50) { throw 'Local frontend did not start.' }
    Write-Host "Results: $stage"
    Measure-Run
} finally {
    if (-not $server.HasExited) { $server.Kill($true); $server.WaitForExit() }
}







