# BreadOS decision log

## D001 — Fresh native OS workspace

The starting workspace contained only the master prompt. Create an original Rust UEFI loader and freestanding kernel rather than adopting an existing kernel.

## D002 — Toolchain and boot VM

Pin Rust 1.95.0 and use x86-64 UEFI, one QEMU `pc` core, TCG, and OVMF first. QEMU 11.1.1 is the intended pin; the available Windows package index was observed at 11.1.0 and is not treated as satisfying the pin.

## D003 — Boot protocol

Use a small versioned `repr(C)` boot structure, normalize UEFI memory descriptors into project-owned records, and document that handoff-owned pages and framebuffer remain reserved. Use the reviewed `uefi` crate for firmware protocol calls and its exit helper for current-map-key handling and retry.

## D004 — Temporary kernel renderer

Permit a kernel-hosted, CPU-rendered screen only as M001 bring-up. It is not an isolated desktop; keep the display buffer described through a replaceable interface.

## D005 — Desktop bring-up boundary

Implement the M002 taskbar, two useful views, window policy, and Notes state in
a small `no_std` library so host tests can exercise transitions independently.
The kernel still owns input polling and framebuffer writes. Mark all views as
sharing the kernel address space until M003 adds and fault-tests process
isolation. Keep unsupported network and persistent storage visibly unavailable.
