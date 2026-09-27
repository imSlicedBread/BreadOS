# BreadOS v001

BreadOS is an original operating-system prototype for U.S. bakeries. This repository begins with the UEFI loader and a kernel-hosted desktop bring-up; it is not a Linux distribution and makes no Windows binary compatibility claim.

## Requirements

- Rust 1.95.0 MSVC toolchain. `rust-toolchain.toml` pins the compiler and installs the `x86_64-unknown-uefi` and `x86_64-unknown-none` targets.
- Python 3 for the standard-library FAT12 image builder.
- QEMU 11.1.1 and EDK2/OVMF code and variable-store images for guest boot. Upstream QEMU provides source releases and Windows/MSYS2 build guidance; the Windows package index available during initial setup exposed 11.1.0, so do not treat that package as the pinned 11.1.1 build.

Set `BREADOS_QEMU`, `BREADOS_OVMF_CODE`, and `BREADOS_OVMF_VARS` to the emulator and firmware files. Firmware variables are copied into `out/run/` for each run.

For a reproducible Windows guest setup, install the official QEMU **11.1.1**
release using the available Windows or MSYS2 route in the [QEMU downloads
guide](https://www.qemu.org/download/), and use an x86-64 OVMF code image plus
a writable variables template as described in the [TianoCore OVMF
guide](https://www.tianocore.org/tianocore-wiki.github.io/development/tutorials-howto/how_to_run_ovmf.html).
Set the three `BREADOS_*` variables above to those exact files. The current
host's installed QEMU is 11.1.0, so the recorded guest evidence is provisional
until the smoke suite is repeated with 11.1.1.

## Build, test, and run

```powershell
cargo xtask check-env
cargo fmt --all -- --check
cargo test -p breados-boot -p breados-desktop
cargo xtask build
cargo xtask image
cargo xtask smoke
cargo xtask run
```

The raw FAT12 UEFI image is `out/BreadOS-v001.img`. The bounded smoke run
captures serial output, asserts loader and kernel milestones, sends QMP keyboard
and pointer input, and stores actual guest screenshots under `out/run/`.
`run` opens the QEMU display and writes serial diagnostics to
`out/run/serial.log`. The validated smoke host had QEMU 11.1.0; QEMU 11.1.1 is
the requested pin and remains a repeat-test gate.

## Current limits

- M001 boot and M002 desktop checks, including title-bar maximize/restore, pass on QEMU 11.1.0 with OVMF. Keyboard snapping has host-side coverage; repeat guest smoke with the requested QEMU 11.1.1 pin.
- The initial kernel owns the entire address space. This is not a protected microkernel desktop; M003 process isolation and fault containment are unimplemented.
- Notes are RAM-only and clearly state that changes are lost on reboot. There is no persistent guest filesystem, network stack, bakery workflow, or production control.
- The tray reports `NET OFF`; networking, Files, Settings, notifications, and touch support are unavailable.
- GPT is used as a development assistant only. BreadOS does not yet host an agent runtime or call an inference provider.

See [STATUS.md](STATUS.md), [docs/DESKTOP.md](docs/DESKTOP.md),
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md),
[docs/DEPENDENCIES.md](docs/DEPENDENCIES.md), and
[docs/DECISIONS.md](docs/DECISIONS.md).
