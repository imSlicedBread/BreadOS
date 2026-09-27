# BreadOS milestone status

Current artifact: `out/BreadOS-v001.img` (1,474,560-byte FAT12 image). The
guest evidence below is from Windows with Rust 1.95.0, QEMU 11.1.0, OVMF,
one-core TCG, and 256 MiB RAM. The requested QEMU 11.1.1 pin remains unmet.

| Capability | Status | Evidence |
|---|---|---|
| Rust workspace, pinned compiler, cross targets | Compiled | Rust 1.95.0; `cargo xtask build` produces EFI and kernel binaries |
| Versioned handoff, memory ranges, framebuffer validation/clipping | Host-tested | 4 `breados-boot` tests pass |
| UEFI loader, ExitBootServices handoff, kernel exception setup | Guest-tested | Bounded smoke log records loader validation, ExitBootServices, kernel entry, IDT setup |
| Timer, PS/2 keyboard and pointer, framebuffer desktop | Guest-tested | QEMU serial milestones; native loop continues after firmware handoff |
| M002 launcher, About/System Information, Notes, focus and Notes editing | Guest-tested | QMP smoke checks F1/F2, keyboard focus, typing, cursor movement, backspace and Alt+F4 |
| Window move, resize, minimize and taskbar restore | Guest-tested | QMP pointer events emit completion/restore markers in serial log |
| Maximize/restore | Guest-tested | QMP clicks both title-bar states; smoke requires two `maximize toggled` serial events |
| Window bounds and keyboard snapping | Host-tested | `breados-desktop` geometry and snap tests; Win+Left/Right path compiled |
| Guest appearance | Manually observed | Actual OVMF/QEMU screenshots inspected: Notes and About/System Information |
| System tray status | Guest-tested | Visible `NET OFF`; clicking opens System Information, asserted in smoke |
| v001 image | Built and guest-tested | `cargo xtask image`, then `cargo xtask smoke`; image saved under `out/` |
| M003 protected applications and fault isolation | Not implemented | Desktop windows share the kernel address space; no user mode or process isolation |

## Current gates and next milestone

- Repeat the same bounded boot and desktop smoke with QEMU **11.1.1**. The
  available Windows installation is QEMU 11.1.0; do not record the requested
  emulator pin as satisfied.
- M003 requires user-mode address spaces, scheduling, syscalls, bounded IPC,
  resource handles, and fault-isolation tests. It has not started.
- Storage, networking, persistence, arbitrary-device support, bakery workflows,
  production controls, GPT inference, and agent orchestration are not present.
- Keyboard Win+Left/Right snapping has host state/geometry tests but is not yet
  part of the guest smoke assertions.

## Reproduction

```powershell
cargo fmt --all -- --check
cargo test -p breados-boot -p breados-desktop
cargo xtask build
cargo xtask image
cargo xtask smoke
cargo xtask run
```

The smoke run refreshes the private OVMF variables copy, captures serial output,
and creates genuine guest screenshots under `out/run/`. See
[`docs/DESKTOP.md`](docs/DESKTOP.md), [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md),
and [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md).
