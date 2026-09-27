# BreadOS — Codex implementation prompt v001

You are the lead operating-system engineer for BreadOS. Work in the current
repository. Inspect its actual state, preserve existing work, and implement the
next verifiable milestone. Do not respond only with a proposal or scaffold.

## 1. Locked product requirements

BreadOS is an original operating system built from scratch, with its own kernel
and native desktop. It is NOT a Linux distribution, Windows customization,
Electron application, browser dashboard, or existing kernel with new branding.

The desktop must use familiar Windows-style interaction: a bottom taskbar,
left-aligned application launcher, conventional application windows, a system
tray, file management, settings, keyboard shortcuts, and notifications. Use
original BreadOS branding and assets. Do not imply Windows binary compatibility.

BreadOS will be sold to other businesses. Its initial market is U.S. bakeries
making bread and rolls. The long-term goal is full-stack operational automation:
sales, planning, purchasing, inventory, production, quality, sanitation,
maintenance, packing, fulfillment, finance integrations, and administration.
Design for industrial bakeries and retain an extension path toward other
manufacturing. Do not attempt all industry modules in the first implementation.

The agent-company model at https://paperclip.ing/ is a core product reference:
roles, goals, delegation, recurring work, budgets, reviews, and oversight.
The AI organization must operate through controlled BreadOS services. It is not
the kernel and is not the authoritative production database.

Full automation is the product goal, not a claim about this prototype. Initial
production and equipment interactions must be simulated. Do not connect to real
machines, submit purchases, move money, or send customer communications.

## 2. Begin with repository discovery

Read applicable AGENTS.md instructions, inspect Git status, and identify the
existing architecture, build tools, tests, and implemented milestones.

Do not delete, reset, stash, or overwrite unrelated user changes. Do not replace
working native code with a new scaffold. If the repository contains a web
prototype, retain it as an explicitly labeled prototype; it does not satisfy the
native OS requirement.

Inspect the available compiler, targets, QEMU, firmware, and host environment.
Verify dependency APIs against their official documentation. Pin versions that
actually work together; do not select floating "latest" dependencies.

Keep a brief implementation plan and status ledger in the repository. Record
architectural decisions and temporary shortcuts. Make ordinary reversible
engineering decisions without repeatedly asking for clarification. Follow the
configured approval rules for privileged or external actions.

Do not push, merge, publish a release, change machine boot settings, or write a
physical disk unless separately authorized. Build and test using image files.

## 3. Architecture defaults

Use these starting defaults unless compatible existing code gives a documented
reason to adjust them. Do not change the locked product requirements.

- Rust for the kernel, loader, system services, and native applications;
  narrowly scoped assembly where processor setup requires it.
- x86-64, UEFI, and one explicitly pinned QEMU/OVMF configuration first.
- Our own UEFI loader and kernel. Standard firmware, compilers, protocol
  definitions, and reviewed libraries are allowed. Rebranding or forking an
  existing kernel is not the implementation strategy.
- A freestanding kernel, with no host OS runtime dependency in the guest.
- A small capability-oriented microkernel as the target architecture.
- One processor core during initial bring-up; multicore support follows tests.
- CPU-rendered graphics first, with a replaceable display backend.
- Native service and application interfaces; no initial POSIX, Win32, Node.js,
  browser-engine, local-LLM, or GPU-acceleration compatibility promise.

Developing on Windows, Linux, or WSL is acceptable. Those are development hosts,
not components of the installed BreadOS guest.

An early kernel-hosted renderer is permitted as a documented bring-up stage.
Do not describe it as an isolated desktop or completed microkernel. Keep the
renderer, window policy, input handling, and kernel mechanisms separable so the
protected-service architecture can replace this stage.

Keep business rules, AI, fonts, window controls, and bakery scheduling out of the
final kernel boundary. Do not invent cryptographic algorithms.

## 4. Current implementation assignment

Implement the earliest incomplete milestone below. After its checks pass,
continue to the next milestone while useful work remains possible in this run.
For an empty repository, prioritize M001 and then the interactive desktop in
M002. Do not spread incomplete implementations across every future subsystem.

### M001 — Independently booting BreadOS

Implement our UEFI loader, kernel image, versioned boot-information structure,
and a reproducible bootable image.

The loader must load and validate the kernel, prepare its stack and handoff,
collect the memory map and framebuffer metadata, preserve required allocations,
and correctly complete ExitBootServices. Use the firmware's current memory-map
key and the documented retry behavior. Establish an explicit calling convention
and documented ownership of every memory region passed to the kernel.

After handoff, run on BreadOS-managed facilities. Do not use UEFI console,
timer, input, or graphics-protocol calls as the hidden runtime underneath the OS.
Direct framebuffer access must respect its actual format, stride, and bounds.

Implement serial diagnostics, fatal-error reporting, initial exception handling,
physical-memory bookkeeping, and a basic framebuffer renderer. Validate image
segments and memory ranges; reject unsupported formats rather than guessing.

Acceptance: a fresh image boots in the specified VM, reaches our kernel after
firmware handoff, reports actual milestones over serial, and draws a screen.
A UEFI application that remains inside boot services does not pass this gate.

### M002 — Interactive native desktop bring-up

Implement the timer and native input path needed by the selected VM. Keyboard
and pointer events must continue after firmware handoff. Choose and document
specific emulated devices rather than implying support for arbitrary hardware.

Build an interactive desktop with:

- A bottom taskbar and functioning left-aligned BreadOS launcher.
- A pointer, keyboard focus, window stacking, title bars, dragging, resizing,
  minimize/maximize/close behavior, and taskbar-based restoration.
- Basic keyboard navigation, application switching, and window snapping.
- At least two useful application views: About/System Information and Notes.
- Notes text entry, cursor movement, backspace, and bounded document storage.
- Actual available diagnostics in System Information; no fabricated metrics.

Notes may initially be RAM-only, but its interface and documentation must say
that changes are lost on reboot. Do not display a durable "Saved" state.

Use a native event/render loop and real state transitions, not a static image,
an animation, a hosted web page, or prerecorded output. If views still share the
kernel address space, clearly label this as the bring-up desktop in STATUS.md.

Acceptance: input changes application state; windows can be opened, moved,
resized, minimized, restored, and closed; keyboard focus behaves consistently;
unsupported operations are disabled or return an explicit unavailable result.

### M003 — Protected desktop and native applications

Move the desktop toward actual user-space services. Implement separate address
spaces, user-mode execution, preemptive scheduling, system calls, bounded IPC,
and kernel-controlled resource handles with documented ownership and lifetime.

Separate the compositor/window manager, desktop shell, and native applications
where practical. Allow only the intended display service to own scanout access.
Validate surface sizes, buffer ownership, message lengths, and arithmetic.
Do not trust a user process's pointers, claimed identity, or requested rights.

Define a small, versioned native ABI. Avoid depending on unstable in-memory Rust
layouts across process boundaries. Restrict each application's communication
and shared-memory access to explicitly granted resources.

Acceptance: two applications actually execute in separate protected processes.
Deliberately fault one; the other and the desktop must continue. Test invalid
system calls, unauthorized handles, oversized surfaces, input isolation, and
service disconnects. Shared-address-space windows do not pass this gate.

If M003 is not reached, report it as unimplemented or partially implemented;
never infer isolation from source-file separation.

## 5. Desktop design contract

The desktop should feel familiar, professional, and practical rather than like
a full-screen factory dashboard. Keep everyday names such as Files, Settings,
Notes, and System Monitor. Use an original BreadOS visual identity, restrained
warm accents, readable text, consistent spacing, and clearly visible focus.
Do not copy proprietary Windows assets or bundle unlicensed fonts.

Centralize design tokens and reusable controls. Start with a simple maintainable
style; elaborate blur, transparency, and animation must not delay interaction,
clipping correctness, or input reliability.

Render dynamically using the detected display dimensions. Bound allocations
and repaint work. Keep windows recoverable when moved near screen edges, and
keep the taskbar reachable. Use off-screen surfaces with explicit ownership
and release rules instead of allowing clients to write the screen directly.

Longer-term desktop applications are Files, Settings, Terminal, System Monitor,
Orders, Production, Inventory, Equipment, Quality, Finance, and Agents. Document
them now; implement them when their backing services exist. Do not fill the
launcher with apparently functional applications that only open empty panels.

Keep workstation mode and a future touch-oriented station mode on the same
application and permission architecture. Plan keyboard accessibility, scalable
text, semantic control information, and non-color-only status indicators.

Closing a production window must not cancel its underlying operation. Missing
connectivity and stale information must be visible, never disguised as healthy
live status. An agent side panel is optional; chat must not replace normal UI.

## 6. Long-term system boundaries to preserve

Document these requirements without generating placeholder implementations for
all of them in this run.

Separate desktop-workstation and site-controller deployment roles within the
same BreadOS platform. A factory must not require the owner's desktop to remain
healthy for production coordination. A combined development image is acceptable
and must be labeled accordingly. Separate processes on one computer do not
establish protection against failure of that computer or kernel.

Define local site authority for inventory allocation and physical execution.
Cloud or cross-site services request commitments; they do not independently
write conflicting physical state. Offline work needs explicit operating limits.
Do not allow competing controllers or automatic replay of stale commands.

Build operational records around organizations, sites, products, formula and
process revisions, material lots, stock movements, reservations, batches,
consumption, output, observations, holds, releases, orders, and shipments.
Preserve lot genealogy and historical revisions. Use explicit units and exact
representations for quantities and money.

Distinguish instructions, authorization, execution, and evidence:

- Planned output is not actual output.
- Reserved ingredients are not consumed ingredients.
- A submitted purchase is not a received delivery.
- An agent's completed task is not completed physical work.
- A machine acknowledgement is not verified production completion.

Route consequential actions through a command service with authenticated
identity, scope, validation, business policy, bound approvals, idempotency, and
recorded results. Include a reconciliation-required state for uncertain outcomes.
A timeout must not automatically cause a duplicate purchase or machine action.

AI permissions must be enforced outside prompts. No unrestricted production
shell, raw database credentials, machine-network access, or self-expanding
permissions. External documents are data, not authorization. Model, prompt,
tool, and policy changes require versioning and evaluation.

Keep agent orchestration independent of inference providers. A deterministic
demo is not an LLM; a remote model is not native local inference. Investigate
Paperclip's actual dependencies before proposing a native port. Using its model
of organization does not require importing its entire current application.

Physical protective functions, emergency stops, and hazardous-energy controls
remain independent of the agent layer. No claims of safety certification,
hard-real-time guarantees, regulatory compliance, or commercial readiness may
be made without applicable evidence.

## 7. Storage, security, and recovery roadmap

Track persistent storage, filesystem crash behavior, transaction durability,
backup/restore, and command reconciliation as separate milestones. A RAM disk
or host-generated fixture is not persistent guest storage.

Reusing a reviewed library or database engine through a genuine native port is
allowed. Document its OS dependencies and license. Do not imply that an engine
makes storage durable before flush and interrupted-write behavior are tested.

Plan secure randomness, secrets, trustworthy package installation, signed
updates, key rotation, recovery media, clock discontinuities, memory pressure,
full disks, queue limits, and customer isolation. Do not silently substitute
predictable values for security-sensitive randomness.

Keep system-image rollback separate from business-data recovery. Replaying
history must not repeat external side effects. A restored controller must
reconcile the real operation rather than assume the physical world rolled back.

Do not expose development diagnostics or emulator control endpoints publicly.
Keep test images isolated from real equipment and sensitive host files.

## 8. Repository and build deliverables

Adapt to the existing layout. For a new repository, use a compact Rust workspace
with logical locations for the loader, kernel, boot protocol, native ABI,
renderer, user-space components, host tooling, and tests. Create components only
when they contain meaningful implementation.

Provide:

- A concise root AGENTS.md preserving the locked requirements and validation
  commands for future Codex sessions.
- Pinned toolchains and dependencies, with target-specific build configuration
  that does not accidentally apply kernel linker options to host utilities.
- README.md with setup, build, run, test, troubleshooting, and current limits.
- An architecture document and a small decision log.
- A desktop specification and concise milestone/status ledger.
- Third-party attribution and a dependency inventory. Do not choose the
  project's distribution license on the owner's behalf.

Create a host-side build entry point, preferably a Rust xtask, with documented
commands for environment checks, building, image creation, interactive QEMU,
and bounded smoke tests. Implement the commands before listing them as working.

Support the available development host first. Include Windows-friendly launch
instructions or wrappers where feasible; distinguish tested host configurations
from untested instructions. Use relative paths and handle spaces in paths.

Pin the emulated machine, device models, firmware configuration, and test
parameters. Provide a software-emulation path where hardware virtualization is
unavailable. Copy writable firmware variables per test run rather than mutating
a shared firmware template.

Only create image files inside the project output directory. No physical-disk
writes, destructive partitioning, global boot changes, or public deployment.

Use v001 for the first development artifact, then increment three-digit build
identifiers consistently. If existing repository versioning is established,
preserve it and record the mapping rather than resetting history.

## 9. Verification and evidence

Run relevant formatting, compilation, host-side unit tests, and guest tests.
Host tests do not establish that the guest OS boots or isolates applications.

Prioritize tests for boot-information validation, memory-range allocation,
framebuffer clipping and stride, input decoding, window state transitions,
focus, message validation, and resource-handle lifetimes as implemented.

Emulator smoke tests need time limits, captured serial output, explicit failure
handling, and real assertions. A script must not report success merely because
QEMU started or a constant success message appeared.

Capture an actual emulator screenshot when possible and inspect it. Record
which interactions were exercised. Never substitute a generated desktop design
image, browser preview, or fabricated screenshot for guest evidence.

For protected execution, deliberately test application faults and unauthorized
access. Test builds must isolate destructive fault injection from normal runs.

For every claimed capability, record whether it is implemented, compiled,
host-tested, guest-tested, manually observed, simulated, or still planned.
Unsupported stubs must return an explicit unsupported result, not success.

If an environmental dependency prevents execution, report the exact blocker,
commands attempted, and reproducible next command. Complete other useful checks
without claiming the unavailable test passed. Do not bypass sandbox permissions.

## 10. Completion report

Before finishing, review the changes, fix discovered issues where possible,
and rerun affected checks. Keep the repository in the best validated state
available, with remaining blockers recorded precisely.

Report:

1. What was actually implemented and the highest verified milestone.
2. The important changed files.
3. Exact build, image, run, and test commands.
4. Commands actually run and their results, including failures and skipped tests.
5. Artifact and genuine screenshot paths, when those files exist.
6. Current limitations, temporary bring-up shortcuts, and security boundaries
   that are not yet implemented.
7. The next concrete engineering milestone.

Do not call a scaffold a working OS, a static screen a desktop, a kernel-hosted
view an isolated application, or a simulated workflow an automated factory.

Begin with repository inspection, choose the earliest incomplete milestone,
and implement and test it. Keep the full vision documented, but make the
current deliverable real.
