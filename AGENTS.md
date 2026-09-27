# BreadOS engineering rules

- Treat `BreadOS_Codex_Master_Prompt_v001.md` as the locked product and safety requirements. Read it before changing architecture.
- BreadOS is an original OS. Never replace the loader/kernel with Linux, an existing kernel, a browser shell, or an Electron app.
- Preserve unrelated work. Keep image files inside `out/`; do not write physical disks, change machine boot settings, publish releases, or connect real equipment.
- Report capability status precisely: implemented, compiled, host-tested, guest-tested, manually observed, simulated, or planned. A host test is not guest boot evidence.
- Do not claim M003 isolation until applications execute in separately protected processes and fault isolation is tested.
- Current v001 targets are Rust 1.95.0, x86-64 UEFI, QEMU 11.1.1, one `pc` machine core under TCG, and a FAT12 removable image. If the host only has 11.1.0, record that mismatch and do not mark the requested VM pin satisfied. No installed storage exists in the guest; Notes are volatile.
- Commands: `cargo xtask check-env`, `cargo fmt --all -- --check`, `cargo test -p breados-boot -p breados-desktop`, `cargo xtask build`, `cargo xtask image`, `cargo xtask smoke`, `cargo xtask run`.
- Emulator smoke must be time-bounded, preserve a fresh copy of OVMF variable storage, and assert the actual loader-to-kernel serial milestones.
- Update `STATUS.md` with evidence labels and limitations whenever capabilities change. Guest screenshots must come from the actual QEMU framebuffer and be inspected; host state tests do not establish guest behavior.
