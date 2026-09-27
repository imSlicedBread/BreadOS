$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$image = Join-Path $root 'out/BreadOS-v001.img'
if (-not (Test-Path -LiteralPath $image)) { throw 'Build out/BreadOS-v001.img first.' }
$qemu = $env:BREADOS_QEMU
if (-not $qemu) { $cmd = Get-Command qemu-system-x86_64.exe -ErrorAction SilentlyContinue; if ($cmd) { $qemu = $cmd.Source } }
if (-not $qemu -and (Test-Path -LiteralPath 'C:\Program Files\qemu\qemu-system-x86_64.exe')) { $qemu = 'C:\Program Files\qemu\qemu-system-x86_64.exe' }
if (-not $qemu) { throw 'BLOCKED: QEMU is unavailable; set BREADOS_QEMU after installing the pinned emulator.' }
$code = $env:BREADOS_OVMF_CODE
$varsTemplate = $env:BREADOS_OVMF_VARS
if (-not $code) { $code = 'C:\Program Files\qemu\share\edk2-x86_64-code.fd' }
if (-not $varsTemplate) { $varsTemplate = 'C:\Program Files\qemu\share\edk2-i386-vars.fd' }
if (-not (Test-Path -LiteralPath $code) -or -not (Test-Path -LiteralPath $varsTemplate)) { throw 'BLOCKED: configured OVMF firmware files do not exist.' }
$runDir = Join-Path $root 'out/run'
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$vars = Join-Path $runDir 'OVMF_VARS-smoke.fd'
$serial = Join-Path $runDir 'smoke-serial.log'
$stdout = Join-Path $runDir 'smoke-stdout.log'
$stderr = Join-Path $runDir 'smoke-stderr.log'
$screenshot = Join-Path $runDir 'desktop-smoke.ppm'
$aboutScreenshot = Join-Path $runDir 'desktop-about.ppm'
$qmpPort = 45127
Copy-Item -LiteralPath $varsTemplate -Destination $vars -Force
Remove-Item $serial,$stdout,$stderr,$screenshot,$aboutScreenshot -Force -ErrorAction SilentlyContinue
$qemuArgs = @('-machine','pc,accel=tcg','-m','256M','-smp','1','-device','qemu-xhci,id=xhci','-drive',"if=pflash,format=raw,readonly=on,file=$code",'-drive',"if=pflash,format=raw,file=$vars",'-drive',"if=none,id=boot,format=raw,file=$image",'-device','usb-storage,drive=boot','-boot','order=c','-serial',"file:$serial",'-qmp',"tcp:127.0.0.1:$qmpPort,server=on,wait=off",'-display','sdl','-no-reboot','-no-shutdown')
$quotedQemuArgs = $qemuArgs | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } }
$proc = Start-Process -FilePath $qemu -ArgumentList $quotedQemuArgs -PassThru -WindowStyle Normal -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$qmpClient = $null
$passed = $false
try {
    $required = @('BreadOS loader v001','kernel segments validated and loaded','leaving UEFI boot services','firmware handoff complete','exception vectors installed','PIT/TSC timer calibrated','PS/2 pointer ready','native desktop event loop active')
    $deadline = [DateTime]::UtcNow.AddSeconds(25)
    while ([DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 250
        if (Test-Path -LiteralPath $serial) {
            $content = Get-Content -Raw -LiteralPath $serial
            if (-not [string]::IsNullOrWhiteSpace($content)) {
                $missing = @($required | Where-Object { $content -notmatch [regex]::Escape($_) })
                if ($missing.Count -eq 0) { $passed = $true; break }
            }
        }
        if ($proc.HasExited) { break }
    }
    if (-not $passed) { throw "FAIL: guest did not reach all M002 startup milestones in 25 seconds. Serial: $serial; stderr: $stderr" }

    $qmpClient = [System.Net.Sockets.TcpClient]::new()
    $qmpClient.ReceiveTimeout = 4000
    $connectDeadline = [DateTime]::UtcNow.AddSeconds(5)
    while (-not $qmpClient.Connected -and [DateTime]::UtcNow -lt $connectDeadline) {
        try { $qmpClient.Connect('127.0.0.1',$qmpPort) } catch { Start-Sleep -Milliseconds 100 }
    }
    if (-not $qmpClient.Connected) { throw 'FAIL: QEMU QMP control socket did not accept a connection.' }
    $stream = $qmpClient.GetStream()
    $reader = [System.IO.StreamReader]::new($stream,[System.Text.Encoding]::ASCII,$false,1024,$true)
    $writer = [System.IO.StreamWriter]::new($stream,[System.Text.Encoding]::ASCII,1024,$true); $writer.AutoFlush=$true
    $null = $reader.ReadLine()
    function Send-QmpCommand([string]$id,[string]$command,[hashtable]$arguments) {
        $request = [ordered]@{execute=$command;arguments=$arguments;id=$id} | ConvertTo-Json -Depth 8 -Compress
        $writer.WriteLine($request)
        while ($true) {
            $response=$reader.ReadLine()
            if (-not $response) { throw "QMP connection closed during $id" }
            $object=$response | ConvertFrom-Json
            if ($object.id -eq $id) { if ($object.error) { throw "QMP $id failed: $($object.error.desc)" }; return $object }
        }
    }
    Send-QmpCommand 'caps' 'qmp_capabilities' @{} | Out-Null
    $null = Send-QmpCommand 'open_notes' 'human-monitor-command' @{ 'command-line'='sendkey f2' }
    Start-Sleep -Milliseconds 300
    $null = Send-QmpCommand 'type_h' 'human-monitor-command' @{ 'command-line'='sendkey h' }
    $null = Send-QmpCommand 'type_i' 'human-monitor-command' @{ 'command-line'='sendkey i' }
    $null = Send-QmpCommand 'notes_left' 'human-monitor-command' @{ 'command-line'='sendkey left' }
    $null = Send-QmpCommand 'notes_backspace' 'human-monitor-command' @{ 'command-line'='sendkey backspace' }
    $null = Send-QmpCommand 'notes_right' 'human-monitor-command' @{ 'command-line'='sendkey right' }
    Start-Sleep -Milliseconds 300
    $screenshotQemuPath = $screenshot.Replace('\','/')
    $capture = Send-QmpCommand 'capture' 'human-monitor-command' @{ 'command-line'=('screendump ' + $screenshotQemuPath) }
    if ($capture.return -match 'error|failed|not available') { throw "FAIL: QEMU screenshot command returned: $($capture.return)" }
    $null = Send-QmpCommand 'switch' 'human-monitor-command' @{ 'command-line'='sendkey alt-tab' }
    $null = Send-QmpCommand 'open_about' 'human-monitor-command' @{ 'command-line'='sendkey f1' }
    Start-Sleep -Milliseconds 300
    $aboutQemuPath = $aboutScreenshot.Replace('\','/')
    $aboutCapture = Send-QmpCommand 'capture_about' 'human-monitor-command' @{ 'command-line'=('screendump ' + $aboutQemuPath) }
    if ($aboutCapture.return -match 'error|failed|not available') { throw "FAIL: About screenshot command returned: $($aboutCapture.return)" }
    $null = Send-QmpCommand 'close_about' 'human-monitor-command' @{ 'command-line'='sendkey alt-f4' }
    Start-Sleep -Milliseconds 300
    $null = Send-QmpCommand 'mouse_to_launcher' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=-615}},@{type='rel';data=@{axis='y';value=384}}) }
    $null = Send-QmpCommand 'launcher_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'launcher_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 150
    $null = Send-QmpCommand 'mouse_to_notes_menu' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=105}},@{type='rel';data=@{axis='y';value=-34}}) }
    $null = Send-QmpCommand 'notes_menu_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'notes_menu_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 150
    $null = Send-QmpCommand 'mouse_to_minimize' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=730}},@{type='rel';data=@{axis='y';value=-550}}) }
    $null = Send-QmpCommand 'minimize_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'minimize_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 150
    $null = Send-QmpCommand 'mouse_to_taskbar' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=-690}},@{type='rel';data=@{axis='y';value=581}}) }
    $null = Send-QmpCommand 'restore_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'restore_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 300
    $null = Send-QmpCommand 'mouse_to_maximize' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=714}},@{type='rel';data=@{axis='y';value=-582}}) }
    $null = Send-QmpCommand 'maximize_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'maximize_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 150
    $null = Send-QmpCommand 'mouse_to_restore_maximized' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=360}},@{type='rel';data=@{axis='y';value=-184}}) }
    $null = Send-QmpCommand 'restore_maximized_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'restore_maximized_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 150
    $null = Send-QmpCommand 'mouse_to_resize' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=-332}},@{type='rel';data=@{axis='y';value=561}}) }
    $null = Send-QmpCommand 'resize_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'resize_motion' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=20}},@{type='rel';data=@{axis='y';value=20}}) }
    $null = Send-QmpCommand 'resize_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 150
    $null = Send-QmpCommand 'mouse_to_title' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=-332}},@{type='rel';data=@{axis='y';value=-396}}) }
    $null = Send-QmpCommand 'move_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'move_motion' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=40}},@{type='rel';data=@{axis='y';value=40}}) }
    $null = Send-QmpCommand 'move_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 150
    $null = Send-QmpCommand 'mouse_to_tray' 'input-send-event' @{ events=@(@{type='rel';data=@{axis='x';value=586}},@{type='rel';data=@{axis='y';value=544}}) }
    $null = Send-QmpCommand 'tray_press' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$true;button='left'}}) }
    $null = Send-QmpCommand 'tray_release' 'input-send-event' @{ events=@(@{type='btn';data=@{down=$false;button='left'}}) }
    Start-Sleep -Milliseconds 300
    $content=Get-Content -Raw -LiteralPath $serial
    foreach ($needle in @('Notes opened by F2','volatile Notes text updated','Notes backspace','keyboard focus switched','focused window closed','launcher toggled','launcher selection opened','window minimized','Notes taskbar restore','window resize completed','window move completed','maximize toggled','offline tray opened system info')) {
        if ($content -notmatch [regex]::Escape($needle)) { throw "FAIL: guest interaction did not produce '$needle'. Serial: $serial" }
    }
    if ([regex]::Matches($content, [regex]::Escape('maximize toggled')).Count -lt 2) { throw "FAIL: maximize/restore pair did not complete. Serial: $serial" }
    if (-not (Test-Path -LiteralPath $screenshot) -or (Get-Item -LiteralPath $screenshot).Length -lt 1024) { throw 'FAIL: QEMU did not produce a usable screenshot.' }
    if (-not (Test-Path -LiteralPath $aboutScreenshot) -or (Get-Item -LiteralPath $aboutScreenshot).Length -lt 1024) { throw 'FAIL: QEMU did not produce an About screenshot.' }
    Write-Host $content
    Write-Host "PASS: UEFI handoff, timer, PS/2 input, launcher, Notes editing/focus, window move/resize/minimize/maximize/restore/close, taskbar restore, and offline tray verified. Serial: $serial Screenshot: $screenshot"
} finally {
    if ($qmpClient) { $qmpClient.Dispose() }
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
    $proc.WaitForExit()
}
