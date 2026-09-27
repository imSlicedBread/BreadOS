#![no_std]
#![no_main]

use breados_boot::{
    BootInfo, MemoryRegion, PIXEL_FORMAT_BGRX8, valid_framebuffer, valid_header, valid_memory_map,
    valid_memory_regions,
};
use breados_desktop::{AppId, Desktop, Rect, TASKBAR_HEIGHT, TITLE_HEIGHT};
use core::{panic::PanicInfo, ptr};

#[unsafe(no_mangle)]
pub extern "sysv64" fn kernel_entry(info: *const BootInfo) -> ! {
    serial_init();
    serial("BreadOS kernel v001: firmware handoff complete\r\n");
    if info.is_null() {
        fatal("invalid boot information pointer\r\n");
    }
    let boot = unsafe { &*info };
    if !valid_header(boot) {
        fatal("invalid boot information version\r\n");
    }
    if !valid_framebuffer(&boot.framebuffer) {
        fatal("unsupported framebuffer metadata\r\n");
    }
    if !valid_memory_map(&boot.memory_map) {
        fatal("invalid memory map\r\n");
    }
    let regions = if boot.memory_map.count == 0 {
        &[]
    } else {
        unsafe {
            core::slice::from_raw_parts(
                boot.memory_map.regions as *const MemoryRegion,
                boot.memory_map.count as usize,
            )
        }
    };
    if !valid_memory_regions(&boot.memory_map, regions) {
        fatal("invalid memory region ranges\r\n");
    }
    install_idt();
    serial("BreadOS kernel: exception vectors installed\r\n");
    let available_mib = boot.memory_map.available_pages.saturating_mul(4096) / (1024 * 1024);
    serial_num("BreadOS kernel: conventional memory MiB = ", available_mib);
    let timer_hz =
        calibrate_timer().unwrap_or_else(|| fatal("PIT/TSC timer calibration failed\r\n"));
    serial("BreadOS kernel: PIT/TSC timer calibrated\r\n");
    let mouse_ready = init_ps2_mouse();
    serial(if mouse_ready {
        "BreadOS kernel: PS/2 pointer ready\r\n"
    } else {
        "BreadOS kernel: PS/2 pointer unavailable\r\n"
    });
    serial("BreadOS kernel: native desktop event loop active\r\n");
    desktop_loop(boot, available_mib, timer_hz, mouse_ready)
}

struct Pointer {
    x: i32,
    y: i32,
    buttons: u8,
    packet: [u8; 3],
    packet_len: usize,
    drag: u8,
    drag_app: AppId,
}
impl Pointer {
    fn new(w: i32, h: i32) -> Self {
        Self {
            x: w / 2,
            y: h / 2,
            buttons: 0,
            packet: [0; 3],
            packet_len: 0,
            drag: 0,
            drag_app: AppId::About,
        }
    }
}
struct Keyboard {
    extended: bool,
    release: bool,
    shift: bool,
    ctrl: bool,
    alt: bool,
    win: bool,
    caps: bool,
}
impl Keyboard {
    const fn new() -> Self {
        Self {
            extended: false,
            release: false,
            shift: false,
            ctrl: false,
            alt: false,
            win: false,
            caps: false,
        }
    }
}

fn desktop_loop(info: &BootInfo, available_mib: u64, timer_hz: u64, mouse_ready: bool) -> ! {
    let width = info.framebuffer.width as i32;
    let height = info.framebuffer.height as i32;
    let mut desktop = Desktop::new(width, height);
    let mut pointer = Pointer::new(width, height);
    let mut keyboard = Keyboard::new();
    let mut dirty = true;
    let mut cursor_visible = true;
    let mut last_blink = rdtsc();
    loop {
        let now = rdtsc();
        if now.wrapping_sub(last_blink) >= timer_hz / 2 {
            cursor_visible = !cursor_visible;
            last_blink = now;
            dirty = true;
        }
        while unsafe { inb(0x64) } & 1 != 0 {
            let status = unsafe { inb(0x64) };
            let value = unsafe { inb(0x60) };
            if status & 0x20 != 0 {
                dirty |= mouse_byte(&mut pointer, &mut desktop, value);
            } else {
                dirty |= keyboard_byte(&mut keyboard, &mut desktop, value);
            }
        }
        if dirty {
            render_desktop(
                info,
                &desktop,
                &pointer,
                available_mib,
                timer_hz,
                mouse_ready,
                cursor_visible,
            );
            dirty = false;
        }
        core::hint::spin_loop();
    }
}

fn keyboard_byte(k: &mut Keyboard, d: &mut Desktop, byte: u8) -> bool {
    if byte == 0xe0 {
        k.extended = true;
        return false;
    }
    if byte == 0xe1 {
        k.extended = true;
        return false;
    }
    if byte & 0x80 != 0 {
        k.release = true;
    }
    let code = byte & 0x7f;
    let extended = k.extended;
    let released = k.release;
    k.extended = false;
    k.release = false;
    if !extended && (code == 0x2a || code == 0x36) {
        k.shift = !released;
        return false;
    }
    if code == 0x1d {
        k.ctrl = !released;
        return false;
    }
    if code == 0x38 {
        k.alt = !released;
        return false;
    }
    if extended && code == 0x5b {
        k.win = !released;
        return false;
    }
    if !extended && code == 0x3a && released {
        k.caps = !k.caps;
        return false;
    }
    if released {
        return false;
    }
    if !extended && code == 0x3b {
        d.open(AppId::About);
        serial("BreadOS desktop: About opened by F1\r\n");
        return true;
    }
    if !extended && code == 0x3c {
        d.open(AppId::Notes);
        serial("BreadOS desktop: Notes opened by F2\r\n");
        return true;
    }
    if code == 0x0f {
        if k.alt || k.ctrl {
            d.focus_next();
            serial("BreadOS desktop: keyboard focus switched\r\n");
        } else if d.launcher_open {
            d.launcher_move(true);
        } else {
            d.focus_next();
            serial("BreadOS desktop: keyboard focus switched\r\n");
        }
        return true;
    }
    if code == 0x3e && k.alt {
        d.close(d.focused);
        serial("BreadOS desktop: focused window closed\r\n");
        return true;
    }
    if code == 0x4b && extended {
        if k.win {
            d.snap(d.focused, false);
            serial("BreadOS desktop: window snapped left\r\n");
            return true;
        }
        if d.launcher_open {
            d.launcher_move(false);
            return true;
        }
        if d.focused == AppId::Notes && d.notes_editing {
            d.cursor_left();
            return true;
        }
    }
    if code == 0x4d && extended {
        if k.win {
            d.snap(d.focused, true);
            serial("BreadOS desktop: window snapped right\r\n");
            return true;
        }
        if d.launcher_open {
            d.launcher_move(true);
            return true;
        }
        if d.focused == AppId::Notes && d.notes_editing {
            d.cursor_right();
            return true;
        }
    }
    if code == 0x48 && extended && d.launcher_open {
        d.launcher_move(false);
        return true;
    }
    if code == 0x50 && extended && d.launcher_open {
        d.launcher_move(true);
        return true;
    }
    if !extended && code == 0x01 {
        d.launcher_open = !d.launcher_open;
        d.launcher_selection = 0;
        return true;
    }
    if !extended && code == 0x1c {
        if d.launcher_open {
            d.launcher_activate();
            return true;
        }
        if d.focused == AppId::Notes && d.notes_editing {
            return insert_note(d, b'\n');
        }
    }
    if d.focused == AppId::Notes && d.notes_editing {
        if !extended && code == 0x0e {
            let changed = d.backspace();
            if changed {
                serial("BreadOS desktop: Notes backspace\r\n");
            }
            return changed;
        }
        if extended && code == 0x47 {
            d.cursor_home();
            return true;
        }
        if extended && code == 0x4f {
            d.cursor_end();
            return true;
        }
        if extended && code == 0x53 {
            return d.delete();
        }
        if let Some(ch) = scan_to_ascii(code, k.shift, k.caps) {
            return insert_note(d, ch);
        }
    }
    false
}

fn insert_note(d: &mut Desktop, ch: u8) -> bool {
    let changed = d.insert(ch);
    if changed {
        serial("BreadOS desktop: volatile Notes text updated\r\n");
    }
    changed
}

fn scan_to_ascii(code: u8, shift: bool, caps: bool) -> Option<u8> {
    let (plain, shifted) = match code {
        0x02 => (b'1', b'!'),
        0x03 => (b'2', b'@'),
        0x04 => (b'3', b'#'),
        0x05 => (b'4', b'$'),
        0x06 => (b'5', b'%'),
        0x07 => (b'6', b'^'),
        0x08 => (b'7', b'&'),
        0x09 => (b'8', b'*'),
        0x0a => (b'9', b'('),
        0x0b => (b'0', b')'),
        0x0c => (b'-', b'_'),
        0x0d => (b'=', b'+'),
        0x10 => (b'q', b'Q'),
        0x11 => (b'w', b'W'),
        0x12 => (b'e', b'E'),
        0x13 => (b'r', b'R'),
        0x14 => (b't', b'T'),
        0x15 => (b'y', b'Y'),
        0x16 => (b'u', b'U'),
        0x17 => (b'i', b'I'),
        0x18 => (b'o', b'O'),
        0x19 => (b'p', b'P'),
        0x1a => (b'[', b'{'),
        0x1b => (b']', b'}'),
        0x1e => (b'a', b'A'),
        0x1f => (b's', b'S'),
        0x20 => (b'd', b'D'),
        0x21 => (b'f', b'F'),
        0x22 => (b'g', b'G'),
        0x23 => (b'h', b'H'),
        0x24 => (b'j', b'J'),
        0x25 => (b'k', b'K'),
        0x26 => (b'l', b'L'),
        0x27 => (b';', b':'),
        0x28 => (b'\'', b'"'),
        0x29 => (b'`', b'~'),
        0x2b => (b'\\', b'|'),
        0x2c => (b'z', b'Z'),
        0x2d => (b'x', b'X'),
        0x2e => (b'c', b'C'),
        0x2f => (b'v', b'V'),
        0x30 => (b'b', b'B'),
        0x31 => (b'n', b'N'),
        0x32 => (b'm', b'M'),
        0x33 => (b',', b'<'),
        0x34 => (b'.', b'>'),
        0x35 => (b'/', b'?'),
        0x39 => (b' ', b' '),
        _ => return None,
    };
    if plain.is_ascii_lowercase() {
        Some(if shift ^ caps { shifted } else { plain })
    } else {
        Some(if shift { shifted } else { plain })
    }
}

fn mouse_byte(p: &mut Pointer, d: &mut Desktop, byte: u8) -> bool {
    if p.packet_len == 0 && byte & 0x08 == 0 {
        return false;
    }
    p.packet[p.packet_len] = byte;
    p.packet_len += 1;
    if p.packet_len < 3 {
        return false;
    }
    p.packet_len = 0;
    let old_x = p.x;
    let old_y = p.y;
    let old_buttons = p.buttons;
    if p.packet[0] & 0xc0 == 0 {
        p.x =
            p.x.saturating_add(p.packet[1] as i8 as i32)
                .clamp(0, d.width - 1);
    }
    if p.packet[0] & 0xc0 == 0 {
        p.y =
            p.y.saturating_sub(p.packet[2] as i8 as i32)
                .clamp(0, d.height - 1);
    }
    p.buttons = p.packet[0] & 7;
    let mut changed = p.x != old_x || p.y != old_y || p.buttons != old_buttons;
    if old_buttons & 1 == 0 && p.buttons & 1 != 0 {
        changed |= pointer_press(p, d);
    }
    if old_buttons & 1 != 0 && p.buttons & 1 == 0 {
        if p.drag == 1 {
            serial("BreadOS desktop: window move completed\r\n");
        } else if p.drag == 2 {
            serial("BreadOS desktop: window resize completed\r\n");
        }
        p.drag = 0;
    }
    if p.buttons & 1 != 0 && p.drag != 0 {
        if p.drag == 1 {
            d.move_window(p.drag_app, p.x - old_x, p.y - old_y);
        } else if p.drag == 2 {
            d.resize_window(p.drag_app, p.x - old_x, p.y - old_y);
        }
        changed = true;
    }
    changed
}

fn pointer_press(p: &mut Pointer, d: &mut Desktop) -> bool {
    serial_num("BreadOS desktop: pointer x=", p.x.max(0) as u64);
    serial_num("BreadOS desktop: pointer y=", p.y.max(0) as u64);
    let bar_y = d.height - TASKBAR_HEIGHT;
    if d.launcher_open && p.x >= 8 && p.x < 290 && p.y >= bar_y - 78 && p.y < bar_y {
        if p.y < bar_y - 50 {
            d.launcher_selection = 0;
        } else {
            d.launcher_selection = 1;
        }
        d.launcher_activate();
        serial("BreadOS desktop: launcher selection opened\r\n");
        return true;
    }
    if p.y >= bar_y {
        if p.x < 48 {
            d.launcher_open = !d.launcher_open;
            d.launcher_selection = 0;
            serial("BreadOS desktop: launcher toggled\r\n");
        } else if p.x < 132 {
            d.restore_or_focus(AppId::About);
            serial("BreadOS desktop: About taskbar restore\r\n");
        } else if p.x < 220 {
            d.restore_or_focus(AppId::Notes);
            serial("BreadOS desktop: Notes taskbar restore\r\n");
        } else if p.x >= d.width - 104 {
            d.restore_or_focus(AppId::About);
            serial("BreadOS desktop: offline tray opened system info\r\n");
        }
        return true;
    }
    d.launcher_open = false;
    let hit = [
        d.focused,
        if d.focused == AppId::About {
            AppId::Notes
        } else {
            AppId::About
        },
    ]
    .into_iter()
    .find(|app| d.window(*app).visible() && contains(d.window(*app).rect, p.x, p.y));
    let Some(app) = hit else {
        return true;
    };
    d.focus(app);
    let w = *d.window(app);
    let r = w.rect;
    if p.y < r.y + TITLE_HEIGHT {
        if p.x >= r.x + r.width - 24 {
            d.close(app);
            serial("BreadOS desktop: window closed\r\n");
            return true;
        }
        if p.x >= r.x + r.width - 48 {
            d.maximize_toggle(app);
            serial("BreadOS desktop: maximize toggled\r\n");
            return true;
        }
        if p.x >= r.x + r.width - 72 {
            d.minimize(app);
            serial("BreadOS desktop: window minimized\r\n");
            return true;
        }
        p.drag = 1;
        p.drag_app = app;
        return true;
    }
    if p.x >= r.x + r.width - 16 && p.y >= r.y + r.height - 16 {
        p.drag = 2;
        p.drag_app = app;
        return true;
    }
    if app == AppId::Notes {
        d.notes_editing = true;
    }
    true
}

fn contains(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && y >= r.y && x < r.x + r.width && y < r.y + r.height
}

fn calibrate_timer() -> Option<u64> {
    unsafe {
        let gate = (inb(0x61) & !0x02) | 0x01;
        outb(0x43, 0xb6);
        outb(0x42, 0xa9);
        outb(0x42, 0x04); // channel 2, mode 3, divisor 1193 (~1 kHz)
        outb(0x61, gate);
    }
    let mut last = unsafe { inb(0x61) } & 0x20;
    let start = rdtsc();
    for _ in 0..20 {
        let mut spins = 0u32;
        while unsafe { inb(0x61) } & 0x20 == last {
            core::hint::spin_loop();
            spins += 1;
            if spins == 10_000_000 {
                return None;
            }
        }
        last ^= 0x20;
    }
    let elapsed = rdtsc().wrapping_sub(start);
    let hz = elapsed.checked_mul(100)?;
    if !(1_000_000..=20_000_000_000).contains(&hz) {
        None
    } else {
        Some(hz)
    }
}

fn init_ps2_mouse() -> bool {
    unsafe {
        if !wait_input_empty() {
            return false;
        }
        outb(0x64, 0xa8);
        if !wait_input_empty() {
            return false;
        }
        outb(0x64, 0x20);
        if !wait_output_full() {
            return false;
        }
        let config = (inb(0x60) & !0x03) & !0x20;
        if !wait_input_empty() {
            return false;
        }
        outb(0x64, 0x60);
        if !wait_input_empty() {
            return false;
        }
        outb(0x60, config);
        if !wait_input_empty() {
            return false;
        }
        outb(0x64, 0xd4);
        if !wait_input_empty() {
            return false;
        }
        outb(0x60, 0xf4);
        if !wait_output_full() {
            return false;
        }
        inb(0x60) == 0xfa
    }
}

unsafe fn wait_input_empty() -> bool {
    for _ in 0..100_000 {
        if unsafe { inb(0x64) } & 2 == 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}
unsafe fn wait_output_full() -> bool {
    for _ in 0..100_000 {
        if unsafe { inb(0x64) } & 1 != 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    }
    value
}
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}
fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!("lfence; rdtsc", out("eax") low, out("edx") high, options(nomem, nostack));
    }
    ((high as u64) << 32) | low as u64
}

#[derive(Clone, Copy)]
struct Renderer {
    pixels: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
    bgr: bool,
}
impl Renderer {
    fn color(self, r: u8, g: u8, b: u8) -> u32 {
        if self.bgr {
            ((r as u32) << 16) | ((g as u32) << 8) | b as u32
        } else {
            ((b as u32) << 16) | ((g as u32) << 8) | r as u32
        }
    }
    fn pixel(self, x: usize, y: usize, color: u32) {
        if x < self.width
            && y < self.height
            && y.saturating_mul(self.stride).saturating_add(x)
                < self.stride.saturating_mul(self.height)
        {
            unsafe {
                ptr::write_volatile(self.pixels.add(y * self.stride + x), color);
            }
        }
    }
    fn rect(self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let Some((x, y, w, h)) = breados_boot::clipped_rect(
            x.max(0) as usize,
            y.max(0) as usize,
            w as usize,
            h as usize,
            self.width,
            self.height,
        ) else {
            return;
        };
        for yy in y..y + h {
            for xx in x..x + w {
                self.pixel(xx, yy, color);
            }
        }
    }
    fn outline(self, r: Rect, color: u32) {
        self.rect(r.x, r.y, r.width, 1, color);
        self.rect(r.x, r.y + r.height - 1, r.width, 1, color);
        self.rect(r.x, r.y, 1, r.height, color);
        self.rect(r.x + r.width - 1, r.y, 1, r.height, color);
    }
    fn text(self, x: i32, y: i32, text: &str, color: u32) {
        self.bytes(x, y, text.as_bytes(), color);
    }
    fn bytes(self, mut x: i32, y: i32, text: &[u8], color: u32) {
        for byte in text.iter().copied() {
            if x >= self.width as i32 {
                break;
            }
            self.glyph(x, y, byte, color);
            x += 12;
        }
    }
    fn glyph(self, x: i32, y: i32, byte: u8, color: u32) {
        let rows = glyph(byte);
        for (gy, row) in rows.iter().enumerate() {
            for gx in 0..5 {
                if row & (1 << (4 - gx)) != 0 && x + gx as i32 * 2 >= 0 && y + gy as i32 * 2 >= 0 {
                    let px = x + gx as i32 * 2;
                    let py = y + gy as i32 * 2;
                    self.pixel(px as usize, py as usize, color);
                    self.pixel((px + 1) as usize, py as usize, color);
                    self.pixel(px as usize, (py + 1) as usize, color);
                    self.pixel((px + 1) as usize, (py + 1) as usize, color);
                }
            }
        }
    }
}

fn render_desktop(
    info: &BootInfo,
    d: &Desktop,
    p: &Pointer,
    available_mib: u64,
    timer_hz: u64,
    mouse_ready: bool,
    cursor_visible: bool,
) {
    let fb = info.framebuffer;
    let r = Renderer {
        pixels: fb.base as *mut u32,
        width: fb.width as usize,
        height: fb.height as usize,
        stride: fb.stride_pixels as usize,
        bgr: fb.pixel_format == PIXEL_FORMAT_BGRX8,
    };
    let bg = r.color(34, 28, 24);
    let panel = r.color(48, 39, 33);
    let panel_hi = r.color(64, 52, 44);
    let accent = r.color(197, 116, 57);
    let text = r.color(248, 240, 229);
    let body = r.color(245, 240, 232);
    let body_text = r.color(44, 38, 33);
    let line = r.color(201, 190, 174);
    r.rect(0, 0, d.width, d.height, bg);
    r.rect(
        0,
        0,
        d.width,
        d.height - TASKBAR_HEIGHT,
        r.color(39, 32, 27),
    );
    if d.window(AppId::About).visible() && d.focused != AppId::About {
        draw_window(
            r,
            d,
            AppId::About,
            body,
            body_text,
            line,
            panel,
            panel_hi,
            accent,
            text,
            available_mib,
            timer_hz,
            mouse_ready,
            cursor_visible,
        );
    }
    if d.window(AppId::Notes).visible() && d.focused != AppId::Notes {
        draw_window(
            r,
            d,
            AppId::Notes,
            body,
            body_text,
            line,
            panel,
            panel_hi,
            accent,
            text,
            available_mib,
            timer_hz,
            mouse_ready,
            cursor_visible,
        );
    }
    if d.window(d.focused).visible() {
        draw_window(
            r,
            d,
            d.focused,
            body,
            body_text,
            line,
            panel,
            panel_hi,
            accent,
            text,
            available_mib,
            timer_hz,
            mouse_ready,
            cursor_visible,
        );
    }
    let bar_y = d.height - TASKBAR_HEIGHT;
    r.rect(0, bar_y, d.width, TASKBAR_HEIGHT, panel);
    r.rect(7, bar_y + 4, 40, 24, accent);
    r.text(14, bar_y + 12, "B", text);
    task_button(
        r,
        48,
        bar_y,
        84,
        "ABOUT",
        d.window(AppId::About).open && !d.window(AppId::About).minimized,
        d.focused == AppId::About,
        body_text,
        text,
        panel_hi,
        accent,
    );
    task_button(
        r,
        132,
        bar_y,
        88,
        "NOTES",
        d.window(AppId::Notes).open && !d.window(AppId::Notes).minimized,
        d.focused == AppId::Notes,
        body_text,
        text,
        panel_hi,
        accent,
    );
    r.text(236, bar_y + 10, "BREADOS", text);
    let tray_x = (d.width - 104).max(224);
    r.rect(tray_x, bar_y + 4, 96, 24, panel_hi);
    r.outline(
        Rect {
            x: tray_x,
            y: bar_y + 4,
            width: 96,
            height: 24,
        },
        body_text,
    );
    r.text(tray_x + 10, bar_y + 12, "NET OFF", text);
    if d.launcher_open {
        draw_launcher(r, d, bar_y, body, body_text, line, accent, text);
    }
    draw_pointer(r, p.x, p.y, text, body_text);
}

fn task_button(
    r: Renderer,
    x: i32,
    y: i32,
    w: i32,
    label: &str,
    open: bool,
    active: bool,
    body: u32,
    text: u32,
    panel: u32,
    accent: u32,
) {
    if active {
        r.rect(x, y + 2, w, TASKBAR_HEIGHT - 4, panel);
        r.rect(x, y + TASKBAR_HEIGHT - 3, w, 2, accent);
    } else if open {
        r.rect(x + 5, y + TASKBAR_HEIGHT - 3, w - 10, 2, accent);
    }
    r.text(x + 10, y + 12, label, if active { text } else { body });
}

fn draw_launcher(
    r: Renderer,
    d: &Desktop,
    bar_y: i32,
    body: u32,
    body_text: u32,
    line: u32,
    accent: u32,
    text: u32,
) {
    let x = 8;
    let y = bar_y - 78;
    r.rect(x + 3, y + 4, 280, 76, r.color(10, 8, 7));
    r.rect(x, y, 280, 76, body);
    r.outline(
        Rect {
            x,
            y,
            width: 280,
            height: 76,
        },
        line,
    );
    r.text(x + 10, y + 6, "BREADOS", body_text);
    r.rect(x + 4, y + 20, 272, 1, line);
    for index in 0..2 {
        let row_y = y + 22 + index * 28;
        if d.launcher_selection == index as usize {
            r.rect(x + 3, row_y, 274, 26, accent);
        }
        r.text(
            x + 14,
            row_y + 4,
            if index == 0 {
                "ABOUT / SYSTEM INFO"
            } else {
                "NOTES"
            },
            if d.launcher_selection == index as usize {
                text
            } else {
                body_text
            },
        );
    }
}

fn draw_window(
    r: Renderer,
    d: &Desktop,
    app: AppId,
    body: u32,
    body_text: u32,
    line: u32,
    panel: u32,
    panel_hi: u32,
    accent: u32,
    text: u32,
    available_mib: u64,
    timer_hz: u64,
    mouse_ready: bool,
    cursor_visible: bool,
) {
    let w = *d.window(app);
    let q = w.rect;
    r.rect(q.x + 5, q.y + 7, q.width, q.height, r.color(14, 11, 9));
    r.rect(q.x, q.y, q.width, q.height, body);
    r.outline(q, line);
    r.rect(
        q.x + 1,
        q.y + 1,
        q.width - 2,
        TITLE_HEIGHT - 1,
        if d.focused == app { accent } else { panel },
    );
    let title = if app == AppId::About {
        "SYSTEM INFO"
    } else {
        "NOTES"
    };
    r.text(q.x + 12, q.y + 7, title, text);
    for i in 0..3 {
        let x = q.x + q.width - 72 + i * 24;
        r.rect(x, q.y + 5, 21, 20, if i == 2 { panel_hi } else { panel });
        r.outline(
            Rect {
                x,
                y: q.y + 5,
                width: 21,
                height: 20,
            },
            panel_hi,
        );
        r.text(
            x + 5,
            q.y + 6,
            match i {
                0 => "-",
                1 => "+",
                _ => "X",
            },
            text,
        );
    }
    if app == AppId::About {
        r.text(q.x + 18, q.y + 48, "BREADOS", body_text);
        r.text(q.x + 18, q.y + 68, "NATIVE OS BRING-UP DESKTOP", body_text);
        r.rect(q.x + 18, q.y + 88, q.width - 36, 1, line);
        r.text(q.x + 18, q.y + 100, "BOOT PROTOCOL: V1", body_text);
        r.text(q.x + 18, q.y + 122, "DISPLAY:", body_text);
        r.text_fixed(
            q.x + 126,
            q.y + 122,
            &fb_dimensions(d.width, d.height),
            body_text,
        );
        r.text(q.x + 18, q.y + 144, "AVAILABLE MEM:", body_text);
        r.text_fixed(q.x + 198, q.y + 144, &number(available_mib), body_text);
        r.text(q.x + 246, q.y + 144, "MIB", body_text);
        r.text(q.x + 18, q.y + 166, "TIMER PIT/TSC:", body_text);
        r.text_fixed(q.x + 186, q.y + 166, &timer_mhz(timer_hz), body_text);
        r.text(q.x + 234, q.y + 166, "MHZ", body_text);
        r.text(q.x + 18, q.y + 188, "POINTER:", body_text);
        r.text(
            q.x + 126,
            q.y + 188,
            if mouse_ready {
                "PS/2 READY"
            } else {
                "UNAVAILABLE"
            },
            body_text,
        );
        r.text(
            q.x + 18,
            q.y + 220,
            "WINDOWS SHARE THE KERNEL ADDRESS SPACE",
            body_text,
        );
        r.text(
            q.x + 18,
            q.y + 240,
            "BRING-UP DESKTOP / NO M003 ISOLATION",
            body_text,
        );
        r.text(
            q.x + 18,
            q.y + 280,
            "F1 ABOUT   F2 NOTES   ALT+TAB SWITCH",
            body_text,
        );
        r.text(
            q.x + 18,
            q.y + 302,
            "WIN+LEFT / RIGHT SNAPS THE ACTIVE WINDOW",
            body_text,
        );
    } else {
        r.text(
            q.x + 18,
            q.y + 48,
            "RAM ONLY - CHANGES ARE LOST ON REBOOT",
            body_text,
        );
        r.text(q.x + 18, q.y + 68, "NO PERSISTENT STORAGE YET", body_text);
        let area = Rect {
            x: q.x + 16,
            y: q.y + 91,
            width: q.width - 32,
            height: q.height - 108,
        };
        r.rect(
            area.x,
            area.y,
            area.width,
            area.height,
            r.color(255, 253, 249),
        );
        r.outline(area, line);
        draw_notes(
            r,
            d,
            area,
            body_text,
            if cursor_visible {
                accent
            } else {
                r.color(255, 253, 249)
            },
        );
    }
}

fn draw_notes(r: Renderer, d: &Desktop, area: Rect, color: u32, caret_color: u32) {
    let cols = ((area.width - 16) / 12).max(1) as usize;
    let rows = ((area.height - 12) / 18).max(1) as usize;
    let (cursor_line, _cursor_col) = text_location(&d.notes[..d.notes_len], d.cursor, cols);
    let first_line = cursor_line.saturating_sub(rows - 1);
    let mut line_no = 0usize;
    let mut col = 0usize;
    for (i, b) in d.notes[..d.notes_len].iter().copied().enumerate() {
        if i == d.cursor {
            draw_caret(r, area, line_no, col, first_line, caret_color);
        }
        if b == b'\n' {
            line_no += 1;
            col = 0;
            continue;
        }
        if line_no >= first_line && line_no - first_line < rows {
            r.glyph(
                area.x + 8 + (col as i32) * 12,
                area.y + 6 + ((line_no - first_line) as i32) * 18,
                b,
                color,
            );
        }
        col += 1;
        if col >= cols {
            line_no += 1;
            col = 0;
        }
    }
    if d.cursor == d.notes_len {
        draw_caret(r, area, line_no, col, first_line, caret_color);
    }
}

fn text_location(text: &[u8], cursor: usize, cols: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for b in text.iter().take(cursor.min(text.len())) {
        if *b == b'\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
            if col >= cols {
                line += 1;
                col = 0;
            }
        }
    }
    (line, col)
}
fn draw_caret(r: Renderer, area: Rect, line: usize, col: usize, first: usize, color: u32) {
    if line >= first {
        let y = area.y + 5 + ((line - first) as i32) * 18;
        let x = area.x + 7 + (col as i32) * 12;
        r.rect(x, y, 2, 16, color);
    }
}
fn draw_pointer(r: Renderer, x: i32, y: i32, white: u32, black: u32) {
    let spans = [
        2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 22, 22, 22, 10, 10, 9, 8, 6, 4,
    ];
    for (row, width) in spans.iter().copied().enumerate() {
        r.rect(x, y + row as i32, width, 1, black);
        if width > 2 {
            r.rect(x + 1, y + row as i32, width - 2, 1, white);
        }
    }
}

fn fb_dimensions(w: i32, h: i32) -> FixedText {
    let mut s = FixedText::new();
    s.number(w as u64);
    s.push(b'X');
    s.number(h as u64);
    s
}
fn number(n: u64) -> FixedText {
    let mut s = FixedText::new();
    s.number(n);
    s
}
fn timer_mhz(hz: u64) -> FixedText {
    let mut s = FixedText::new();
    s.number(hz / 1_000_000);
    s
}
struct FixedText {
    data: [u8; 24],
    len: usize,
}
impl FixedText {
    fn new() -> Self {
        Self {
            data: [0; 24],
            len: 0,
        }
    }
    fn push(&mut self, b: u8) {
        if self.len < self.data.len() {
            self.data[self.len] = b;
            self.len += 1;
        }
    }
    fn number(&mut self, mut n: u64) {
        let mut buf = [0u8; 20];
        let mut len = 0;
        if n == 0 {
            self.push(b'0');
            return;
        }
        while n > 0 && len < 20 {
            buf[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
        while len > 0 {
            len -= 1;
            self.push(buf[len]);
        }
    }
}
impl AsRef<[u8]> for FixedText {
    fn as_ref(&self) -> &[u8] {
        &self.data[..self.len]
    }
}
impl Renderer {
    fn text_fixed(self, x: i32, y: i32, s: &FixedText, color: u32) {
        self.bytes(x, y, s.as_ref(), color);
    }
}

fn glyph(c: u8) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        b'A' => [14, 17, 17, 31, 17, 17, 17],
        b'B' => [30, 17, 17, 30, 17, 17, 30],
        b'C' => [14, 17, 16, 16, 16, 17, 14],
        b'D' => [30, 17, 17, 17, 17, 17, 30],
        b'E' => [31, 16, 16, 30, 16, 16, 31],
        b'F' => [31, 16, 16, 30, 16, 16, 16],
        b'G' => [14, 17, 16, 23, 17, 17, 15],
        b'H' => [17, 17, 17, 31, 17, 17, 17],
        b'I' => [14, 4, 4, 4, 4, 4, 14],
        b'J' => [7, 2, 2, 2, 18, 18, 12],
        b'K' => [17, 18, 20, 24, 20, 18, 17],
        b'L' => [16, 16, 16, 16, 16, 16, 31],
        b'M' => [17, 27, 21, 21, 17, 17, 17],
        b'N' => [17, 25, 21, 19, 17, 17, 17],
        b'O' => [14, 17, 17, 17, 17, 17, 14],
        b'P' => [30, 17, 17, 30, 16, 16, 16],
        b'Q' => [14, 17, 17, 17, 21, 18, 13],
        b'R' => [30, 17, 17, 30, 20, 18, 17],
        b'S' => [15, 16, 16, 14, 1, 1, 30],
        b'T' => [31, 4, 4, 4, 4, 4, 4],
        b'U' => [17, 17, 17, 17, 17, 17, 14],
        b'V' => [17, 17, 17, 17, 17, 10, 4],
        b'W' => [17, 17, 17, 21, 21, 21, 10],
        b'X' => [17, 17, 10, 4, 10, 17, 17],
        b'Y' => [17, 17, 10, 4, 4, 4, 4],
        b'Z' => [31, 1, 2, 4, 8, 16, 31],
        b'0' => [14, 17, 19, 21, 25, 17, 14],
        b'1' => [4, 12, 4, 4, 4, 4, 14],
        b'2' => [14, 17, 1, 2, 4, 8, 31],
        b'3' => [30, 1, 1, 14, 1, 1, 30],
        b'4' => [2, 6, 10, 18, 31, 2, 2],
        b'5' => [31, 16, 16, 30, 1, 1, 30],
        b'6' => [14, 16, 16, 30, 17, 17, 14],
        b'7' => [31, 1, 2, 4, 8, 8, 8],
        b'8' => [14, 17, 17, 14, 17, 17, 14],
        b'9' => [14, 17, 17, 15, 1, 1, 14],
        b':' => [0, 4, 4, 0, 4, 4, 0],
        b';' => [0, 4, 4, 0, 4, 4, 8],
        b'.' => [0, 0, 0, 0, 0, 4, 4],
        b',' => [0, 0, 0, 0, 0, 4, 8],
        b'-' => [0, 0, 0, 31, 0, 0, 0],
        b'_' => [0, 0, 0, 0, 0, 0, 31],
        b'!' => [4, 4, 4, 4, 4, 0, 4],
        b'?' => [14, 17, 1, 2, 4, 0, 4],
        b'/' => [1, 2, 2, 4, 8, 8, 16],
        b'(' => [2, 4, 8, 8, 8, 4, 2],
        b')' => [8, 4, 2, 2, 2, 4, 8],
        b'[' => [14, 8, 8, 8, 8, 8, 14],
        b']' => [14, 2, 2, 2, 2, 2, 14],
        b'+' => [0, 4, 4, 31, 4, 4, 0],
        b'=' => [0, 31, 0, 31, 0, 0, 0],
        b'\'' => [4, 4, 2, 0, 0, 0, 0],
        b'"' => [10, 10, 5, 0, 0, 0, 0],
        b'\\' => [16, 8, 8, 4, 2, 2, 1],
        _ => [0; 7],
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    attrs: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}
impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            attrs: 0x8e,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }
    fn set(&mut self, address: u64, selector: u16) {
        self.offset_low = address as u16;
        self.selector = selector;
        self.offset_mid = (address >> 16) as u16;
        self.offset_high = (address >> 32) as u32;
    }
}
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}
static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];
#[unsafe(no_mangle)]
extern "C" fn breados_fatal_interrupt() -> ! {
    serial("BreadOS kernel: fatal CPU exception\r\n");
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}
fn install_idt() {
    let handler = breados_fatal_interrupt as *const () as u64;
    let selector: u16;
    unsafe {
        core::arch::asm!("mov {0:x}, cs", out(reg) selector, options(nomem, nostack, preserves_flags));
    }
    unsafe {
        let idt_ptr = ptr::addr_of_mut!(IDT);
        for entry in &mut *idt_ptr {
            entry.set(handler, selector);
        }
        let descriptor = IdtPointer {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: idt_ptr as u64,
        };
        core::arch::asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));
    }
}

fn serial_init() {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x3f9u16, in("al") 0u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fbu16, in("al") 0x80u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") 1u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3f9u16, in("al") 0u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fbu16, in("al") 3u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fau16, in("al") 0xc7u8, options(nomem, nostack, preserves_flags));
        core::arch::asm!("out dx, al", in("dx") 0x3fcu16, in("al") 0x0bu8, options(nomem, nostack, preserves_flags));
    }
}
fn serial(message: &str) {
    for b in message.bytes() {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b, options(nomem, nostack, preserves_flags));
        }
    }
}
fn serial_num(prefix: &str, mut n: u64) {
    serial(prefix);
    let mut b = [0u8; 20];
    let mut i = 0;
    if n == 0 {
        serial("0");
    } else {
        while n > 0 {
            b[i] = (n % 10) as u8 + b'0';
            n /= 10;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            serial_byte(b[i]);
        }
    }
    serial("\r\n");
}
fn serial_byte(b: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x3f8u16, in("al") b, options(nomem, nostack, preserves_flags));
    }
}
fn fatal(message: &str) -> ! {
    serial("BreadOS kernel fatal: ");
    serial(message);
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    fatal("panic\r\n");
}
