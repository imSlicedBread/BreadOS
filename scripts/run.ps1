$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$image = Join-Path $root 'out/BreadOS-v001.img'
if (-not (Test-Path -LiteralPath $image)) { throw "Missing $image. Run cargo xtask image first." }

$qemu = $env:BREADOS_QEMU
if (-not $qemu) {
    $cmd = Get-Command qemu-system-x86_64.exe -ErrorAction SilentlyContinue
    if ($cmd) { $qemu = $cmd.Source }
}
if (-not $qemu -and (Test-Path -LiteralPath 'C:\Program Files\qemu\qemu-system-x86_64.exe')) { $qemu = 'C:\Program Files\qemu\qemu-system-x86_64.exe' }
if (-not $qemu) { throw 'QEMU is unavailable. Install the pinned QEMU 11.1.x build, then set BREADOS_QEMU if it is not on PATH.' }

$code = $env:BREADOS_OVMF_CODE
$varsTemplate = $env:BREADOS_OVMF_VARS
if (-not $code) { $code = 'C:\Program Files\qemu\share\edk2-x86_64-code.fd' }
if (-not $varsTemplate) { $varsTemplate = 'C:\Program Files\qemu\share\edk2-i386-vars.fd' }
if (-not (Test-Path -LiteralPath $code) -or -not (Test-Path -LiteralPath $varsTemplate)) { throw 'The configured OVMF firmware paths do not exist.' }

$runDir = Join-Path $root 'out/run'
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$vars = Join-Path $runDir 'OVMF_VARS.fd'
Copy-Item -LiteralPath $varsTemplate -Destination $vars -Force
$serial = Join-Path $runDir 'serial.log'
Remove-Item -LiteralPath $serial -Force -ErrorAction SilentlyContinue

& $qemu -machine pc,accel=tcg -m 256M -smp 1 -device qemu-xhci,id=xhci -drive "if=pflash,format=raw,readonly=on,file=$code" -drive "if=pflash,format=raw,file=$vars" -drive "if=none,id=boot,format=raw,file=$image" -device usb-storage,drive=boot -boot order=c -serial "file:$serial" -monitor none -no-reboot
