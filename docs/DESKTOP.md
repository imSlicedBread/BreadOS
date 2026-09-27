# BreadOS desktop v001

## Implemented bring-up views

The kernel-hosted renderer draws the taskbar, left launcher, pointer, and two
native window views: About/System Information and Notes. The tray displays
`NET OFF` because networking is not implemented; selecting it opens System
Information. Diagnostics are collected from this boot (display dimensions,
UEFI-reported available memory, calibrated PIT/TSC rate, and PS/2 pointer state).

Notes edits a bounded 2 KiB volatile buffer. Its view states that changes are
lost on reboot and that persistent storage is unavailable. It does not claim
that text is saved.

## Interaction

- `Esc` toggles the launcher; arrow keys move its selection and `Enter` opens it.
- `F1` opens System Information; `F2` opens Notes.
- `Alt+Tab` switches focus; `Alt+F4` closes the focused window.
- `Win+Left` and `Win+Right` snap the focused window.
- Drag title bars to move windows; use the lower-right grip to resize.
- Title-bar controls minimize, maximize/restore, and close. Taskbar buttons
  restore or focus the corresponding view.
- Notes supports printable ASCII, Enter, Tab, left/right cursor movement, Home/End,
  Backspace, and Delete. Storage is bounded and no file I/O occurs.

The desktop uses the QEMU `pc` machine's i8042 PS/2 devices and a polled event
loop. Input and window transitions run after `ExitBootServices`. The two views
share the kernel address space; this is not M003 isolation.

## Not yet available

Files, Settings, notifications, networking, persistent documents, scalable
text, touch input, and other bakery applications are not implemented. The tray
reports unavailable networking explicitly. Physical production remains out of
scope; no devices or equipment are controlled.
