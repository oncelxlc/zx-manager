# Build an unsigned x64 package from the existing Tauri release binary.
param(
    [Parameter(Mandatory)][string]$Publisher,
    [string]$Executable = "$PSScriptRoot/../src-tauri/target/release/zx-once.exe",
    [string]$Output = "$PSScriptRoot/../src-tauri/target/zx-once.msix"
)

$ErrorActionPreference = "Stop"
$makeAppx = (Get-Command MakeAppx.exe -ErrorAction SilentlyContinue).Source
if (!$makeAppx) {
    $kit = Get-ItemPropertyValue -LiteralPath 'HKLM:/SOFTWARE/Microsoft/Windows Kits/Installed Roots' -Name KitsRoot10 -ErrorAction SilentlyContinue
    if ($kit) {
        $makeAppx = Get-ChildItem -Path (Join-Path $kit 'bin/*/x64/makeappx.exe') -File -ErrorAction SilentlyContinue |
            Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1 -ExpandProperty FullName
    }
    if (!$makeAppx) { throw "请安装 Windows 10/11 SDK 的 MakeAppx.exe，或将其目录加入 PATH。" }
}
$tauriRoot = (Resolve-Path "$PSScriptRoot/../src-tauri").Path
$binary = (Get-Item -LiteralPath $Executable).FullName
$package = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
if (Test-Path -LiteralPath $package) { throw "输出文件已存在，请指定新的 -Output 路径。" }

$config = Get-Content -Raw -Encoding utf8 -LiteralPath "$tauriRoot/tauri.conf.json" | ConvertFrom-Json
[xml]$manifest = Get-Content -Raw -Encoding utf8 -LiteralPath "$tauriRoot/msix/AppxManifest.xml"
$manifest.Package.Identity.SetAttribute("Publisher", $Publisher)
$manifest.Package.Identity.SetAttribute("Version", "$($config.version).0")
$definition = New-TemporaryFile
$mapping = New-TemporaryFile
try {
    $manifest.Save($definition.FullName)
    $files = @(
        '[Files]'
        ('"{0}" "AppxManifest.xml"' -f $definition.FullName)
        ('"{0}" "zx-once.exe"' -f $binary)
        $loader = Join-Path (Split-Path -Parent $binary) 'WebView2Loader.dll'
        if (Test-Path -LiteralPath $loader -PathType Leaf) { ('"{0}" "WebView2Loader.dll"' -f $loader) }
        foreach ($icon in "StoreLogo.png", "Square150x150Logo.png", "Square44x44Logo.png") {
            ('"{0}\icons\{1}" "Assets\{1}"' -f $tauriRoot, $icon)
        }
    )
    [IO.File]::WriteAllLines($mapping.FullName, [string[]]$files, [Text.Encoding]::UTF8)
    & $makeAppx pack /f $mapping.FullName /p $package /no
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx 打包失败（退出码 $LASTEXITCODE）" }
    Write-Output $package
} finally {
    Remove-Item -LiteralPath $definition.FullName, $mapping.FullName
}
