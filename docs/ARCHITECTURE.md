# BreadOS architecture v001

## Current boot path

The x86-64 UEFI firmware loads `BOOTX64.EFI` from the removable FAT12 image. The Rust UEFI loader reads `KERNEL.ELF`, accepts only little-endian x86-64 ELF64 images, bounds-checks program headers and file/memory ranges, allocates each load segment at its linked physical address, and obtains the GOP framebuffer configuration. It allocates a versioned `BootInfo` record and a fixed-capacity array of normalized memory regions before calling the `uefi` crate's documented `exit_boot_services` helper. That helper obtains the current memory-map key and performs the bounded retry. After handoff, the kernel uses only direct COM1 I/O and framebuffer memory.

The handoff uses the x86-64 System V ABI. `BootInfo` and `MemoryRegion` use explicit `repr(C)` layouts and a magic/version/size header. The framebuffer reports base, byte length, dimensions, pixel stride, and RGBX/BGRX format. The loader, kernel, boot information, memory-region array, and framebuffer are retained. UEFI boot-services allocations are not used after handoff. The kernel validates the handoff and framebuffer arithmetic before scanout writes.

## Bring-up boundary

After handoff, the kernel calibrates PIT channel 2 against the TSC, polls the
QEMU `pc` machine's i8042 keyboard and mouse, and runs a timer-paced framebuffer
desktop loop. A small `breados-desktop` `no_std` state crate owns window,
launcher, focus, taskbar, snap, and bounded Notes state; the kernel handles
device input and rasterization. The window views currently share one kernel
address space. No process separation or fault containment is claimed.

The intended direction remains a capability-oriented microkernel: kernel
mechanisms, compositor/window policy, shell, and native applications are
separate components, with scanout owned only by the display service. Physical
I/O and bakery operations remain simulated until explicit later milestones.

## VM profile

- x86-64 QEMU `pc`, one virtual CPU, TCG software emulation, 256 MiB RAM.
- EDK2/OVMF UEFI firmware, with a private writable variables copy under `out/run` for every VM launch.
- QEMU `pc` machine's emulated i8042 PS/2 keyboard and mouse; no arbitrary hardware compatibility is implied.
- Removable USB storage with a 1.44 MiB FAT12 image. `tools/mkimage.py` writes only beneath `out/` when invoked by the build tooling.
- Serial diagnostics use the emulated 16550 COM1 port at 115200 8N1.

## Later deployment and business boundaries

Future deployments distinguish workstation and site-controller roles. Site authority owns local inventory reservations and physical execution; cloud and cross-site services request commitments. Durable business records must preserve lot genealogy, revisions, exact units and money, and evidence for instructions, approvals, execution, and verification. Consequential actions pass through authenticated, scoped, validated, idempotent command services with reconciliation for uncertain outcomes. AI orchestration and inference remain provider-independent and outside the kernel; model and policy changes require versioning and evaluation. These requirements are documented, not implemented in v001.
