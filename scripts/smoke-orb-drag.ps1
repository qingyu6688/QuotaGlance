param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath
)

$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "浮球拖拽烟测仅支持交互式 Windows 桌面会话。"
}

if (-not (Get-Process -Name explorer -ErrorAction SilentlyContinue)) {
    throw "未检测到交互式 Windows 桌面会话。"
}

$resolvedExecutable = (Resolve-Path -LiteralPath $ExecutablePath).Path
if (Get-Process -Name "quota-glance" -ErrorAction SilentlyContinue) {
    throw "检测到正在运行的 QuotaGlance。请正常退出后再执行烟测。"
}

if (-not ("QuotaGlanceSmokeNative" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;

public static class QuotaGlanceSmokeNative
{
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr handle, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(
        IntPtr handle,
        IntPtr insertAfter,
        int x,
        int y,
        int width,
        int height,
        uint flags
    );

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr handle);

    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(
        uint flags,
        uint x,
        uint y,
        uint data,
        UIntPtr extraInfo
    );

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindow(string className, string title);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
}
'@
}

function Get-SmokeWindowRect([IntPtr]$Handle) {
    $nativeRect = New-Object QuotaGlanceSmokeNative+RECT
    if (-not [QuotaGlanceSmokeNative]::GetWindowRect($Handle, [ref]$nativeRect)) {
        throw "无法读取 QuotaGlance 窗口坐标。"
    }

    [pscustomobject]@{
        x = $nativeRect.Left
        y = $nativeRect.Top
        width = $nativeRect.Right - $nativeRect.Left
        height = $nativeRect.Bottom - $nativeRect.Top
    }
}

function Send-SmokeMouseClick(
    [int]$X,
    [int]$Y,
    [switch]$Double,
    [switch]$Right
) {
    [void][QuotaGlanceSmokeNative]::SetCursorPos($X, $Y)
    $downFlag = if ($Right) { 0x0008 } else { 0x0002 }
    $upFlag = if ($Right) { 0x0010 } else { 0x0004 }
    $count = if ($Double) { 2 } else { 1 }

    for ($index = 0; $index -lt $count; $index++) {
        [QuotaGlanceSmokeNative]::mouse_event($downFlag, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 45
        [QuotaGlanceSmokeNative]::mouse_event($upFlag, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 80
    }
}

function Get-SmokeDescendantProcessIds([int]$RootProcessId) {
    $processes = @(Get-CimInstance Win32_Process)
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $seen = [System.Collections.Generic.HashSet[int]]::new()
    $queue.Enqueue($RootProcessId)

    while ($queue.Count -gt 0) {
        $parentId = $queue.Dequeue()
        foreach ($child in $processes | Where-Object { $_.ParentProcessId -eq $parentId }) {
            $childId = [int]$child.ProcessId
            if ($seen.Add($childId)) {
                $queue.Enqueue($childId)
            }
        }
    }

    @($seen)
}

function New-SmokePreferences {
    [ordered]@{
        schemaVersion = 1
        revision = 0
        locale = "zh-CN"
        theme = "aurora"
        widget = [ordered]@{
            mode = "orb"
            alwaysOnTop = $true
            clickThrough = $false
            selectedQuota = [ordered]@{ limitId = $null; slot = $null }
            boundsByMode = [ordered]@{ orb = $null; card = $null }
        }
        notifications = [ordered]@{
            enabled = $false
            warningRemainingPercent = 50.0
            criticalRemainingPercent = 10.0
            notifyWhenRecovered = $false
        }
        startup = [ordered]@{ launchAtLogin = $false }
        updates = [ordered]@{
            autoCheck = $true
            channel = "stable"
            lastCheckedAt = $null
        }
    }
}

$configDirectory = Join-Path $env:APPDATA "io.github.maorongkang.quotaglance"
$stateFileNames = @("preferences.json", "preferences.json.bak", ".window-state.json")
$backupDirectory = Join-Path $env:TEMP ("quotaglance-smoke-" + [guid]::NewGuid().ToString("N"))
$originalFiles = @{}
$process = $null
$trackedProcessIds = @()
$mouseIsDown = $false
$result = $null

New-Item -ItemType Directory -Path $configDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $backupDirectory | Out-Null

foreach ($fileName in $stateFileNames) {
    $sourcePath = Join-Path $configDirectory $fileName
    $originalFiles[$fileName] = Test-Path -LiteralPath $sourcePath
    if ($originalFiles[$fileName]) {
        Copy-Item -LiteralPath $sourcePath -Destination (Join-Path $backupDirectory $fileName)
    }
}

try {
    $preferencesPath = Join-Path $configDirectory "preferences.json"
    $preferences = if (Test-Path -LiteralPath $preferencesPath) {
        Get-Content -Raw -Encoding UTF8 -LiteralPath $preferencesPath | ConvertFrom-Json
    } else {
        New-SmokePreferences
    }
    $preferences.widget.mode = "orb"
    $preferences.widget.alwaysOnTop = $true
    $preferences.widget.clickThrough = $false
    $preferencesJson = $preferences | ConvertTo-Json -Depth 12
    [IO.File]::WriteAllText($preferencesPath, $preferencesJson, [Text.UTF8Encoding]::new($false))

    $process = Start-Process -FilePath $resolvedExecutable -PassThru
    $startupDeadline = [DateTime]::UtcNow.AddSeconds(25)
    do {
        Start-Sleep -Milliseconds 250
        $process.Refresh()
    } until (
        $process.HasExited -or
        $process.MainWindowHandle -ne 0 -or
        [DateTime]::UtcNow -ge $startupDeadline
    )

    if ($process.HasExited -or $process.MainWindowHandle -eq 0) {
        throw "QuotaGlance 启动后未出现主窗口。"
    }

    $handle = $process.MainWindowHandle
    [void][QuotaGlanceSmokeNative]::SetWindowPos($handle, [IntPtr]::Zero, 220, 180, 0, 0, 0x0005)
    [void][QuotaGlanceSmokeNative]::SetForegroundWindow($handle)
    Start-Sleep -Milliseconds 500

    $initialOrb = Get-SmokeWindowRect $handle
    if (
        [Math]::Abs($initialOrb.width - 136) -gt 3 -or
        [Math]::Abs($initialOrb.height - 136) -gt 3
    ) {
        throw "浮球尺寸异常：$($initialOrb.width) × $($initialOrb.height)。"
    }

    $startX = $initialOrb.x + [int]($initialOrb.width / 2)
    $startY = $initialOrb.y + [int]($initialOrb.height / 2)
    [void][QuotaGlanceSmokeNative]::SetCursorPos($startX, $startY)
    [QuotaGlanceSmokeNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    $mouseIsDown = $true
    for ($step = 1; $step -le 12; $step++) {
        [void][QuotaGlanceSmokeNative]::SetCursorPos($startX + 8 * $step, $startY + 6 * $step)
        Start-Sleep -Milliseconds 55
    }
    [QuotaGlanceSmokeNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    $mouseIsDown = $false
    Start-Sleep -Milliseconds 850

    $draggedOrb = Get-SmokeWindowRect $handle
    $deltaX = $draggedOrb.x - $initialOrb.x
    $deltaY = $draggedOrb.y - $initialOrb.y
    $distance = [Math]::Round([Math]::Sqrt($deltaX * $deltaX + $deltaY * $deltaY), 2)
    if ($distance -lt 40) {
        throw "真实鼠标拖动未移动窗口，位移仅 $distance px。"
    }
    if ($draggedOrb.width -ne $initialOrb.width -or $draggedOrb.height -ne $initialOrb.height) {
        throw "拖动过程中浮球尺寸发生变化。"
    }

    [void][QuotaGlanceSmokeNative]::SetForegroundWindow($handle)
    Send-SmokeMouseClick ($draggedOrb.x + 68) ($draggedOrb.y + 68) -Double
    Start-Sleep -Milliseconds 900
    $expandedCard = Get-SmokeWindowRect $handle
    if (
        [Math]::Abs($expandedCard.width - 320) -gt 3 -or
        [Math]::Abs($expandedCard.height - 320) -gt 3
    ) {
        throw "拖动后双击未展开卡片。"
    }

    [void][QuotaGlanceSmokeNative]::SetForegroundWindow($handle)
    Send-SmokeMouseClick ($expandedCard.x + 160) ($expandedCard.y + 190) -Double
    Start-Sleep -Milliseconds 900
    $restoredOrb = Get-SmokeWindowRect $handle
    if ([Math]::Abs($restoredOrb.width - 136) -gt 3) {
        throw "卡片未收起为浮球。"
    }

    $trackedProcessIds = @($process.Id) + @(Get-SmokeDescendantProcessIds $process.Id)
    $visibleDescendantIds = @(
        $trackedProcessIds | Where-Object {
            $_ -ne $process.Id -and
            (Get-Process -Id $_ -ErrorAction SilentlyContinue).MainWindowHandle -ne 0
        }
    )
    if ($visibleDescendantIds.Count -ne 0) {
        throw "检测到可见子进程窗口：$($visibleDescendantIds -join ', ')。"
    }

    [void][QuotaGlanceSmokeNative]::SetForegroundWindow($handle)
    Send-SmokeMouseClick ($restoredOrb.x + 68) ($restoredOrb.y + 68) -Right
    $menuHandle = [IntPtr]::Zero
    $menuDeadline = [DateTime]::UtcNow.AddSeconds(4)
    do {
        Start-Sleep -Milliseconds 100
        $candidate = [QuotaGlanceSmokeNative]::FindWindow("#32768", $null)
        if ($candidate -ne [IntPtr]::Zero) {
            [uint32]$menuProcessId = 0
            [void][QuotaGlanceSmokeNative]::GetWindowThreadProcessId($candidate, [ref]$menuProcessId)
            if ($menuProcessId -eq $process.Id) {
                $menuHandle = $candidate
            }
        }
    } until ($menuHandle -ne [IntPtr]::Zero -or [DateTime]::UtcNow -ge $menuDeadline)

    if ($menuHandle -eq [IntPtr]::Zero) {
        throw "未检测到浮球原生右键菜单。"
    }

    $contextMenu = Get-SmokeWindowRect $menuHandle
    Send-SmokeMouseClick `
        ($contextMenu.x + [int]($contextMenu.width / 2)) `
        ($contextMenu.y + $contextMenu.height - 14)
    if (-not $process.WaitForExit(10000)) {
        throw "点击右键菜单中的退出项后，应用未在 10 秒内结束。"
    }

    Start-Sleep -Milliseconds 900
    $remainingProcessIds = @(
        $trackedProcessIds | Where-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue }
    )
    if ($remainingProcessIds.Count -ne 0) {
        throw "退出后仍有残留进程：$($remainingProcessIds -join ', ')。"
    }

    $result = [pscustomobject]@{
        executable = $resolvedExecutable
        initialOrb = $initialOrb
        draggedOrb = $draggedOrb
        delta = [pscustomobject]@{ x = $deltaX; y = $deltaY; distance = $distance }
        expandedCard = $expandedCard
        restoredOrb = $restoredOrb
        contextMenu = $contextMenu
        descendantCount = $trackedProcessIds.Count - 1
        visibleDescendantWindows = $visibleDescendantIds.Count
        gracefulExit = $true
        remainingProcesses = $remainingProcessIds.Count
        result = "passed"
    }
} finally {
    if ($mouseIsDown) {
        [QuotaGlanceSmokeNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    }
    if ($null -ne $process -and -not $process.HasExited) {
        foreach ($processId in @(Get-SmokeDescendantProcessIds $process.Id)) {
            Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
        }
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 400
    }

    foreach ($fileName in $stateFileNames) {
        $targetPath = Join-Path $configDirectory $fileName
        $backupPath = Join-Path $backupDirectory $fileName
        if ($originalFiles[$fileName]) {
            Copy-Item -LiteralPath $backupPath -Destination $targetPath -Force
        } elseif (Test-Path -LiteralPath $targetPath) {
            Remove-Item -LiteralPath $targetPath -Force
        }
        if (Test-Path -LiteralPath $backupPath) {
            Remove-Item -LiteralPath $backupPath -Force
        }
    }
    if (Test-Path -LiteralPath $backupDirectory) {
        Remove-Item -LiteralPath $backupDirectory -Force
    }
}

$result | ConvertTo-Json -Depth 5
