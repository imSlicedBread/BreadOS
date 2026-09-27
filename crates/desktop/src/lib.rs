#![no_std]

pub const NOTE_CAPACITY: usize = 2048;
pub const TITLE_HEIGHT: i32 = 30;
pub const TASKBAR_HEIGHT: i32 = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AppId {
    About = 0,
    Notes = 1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub app: AppId,
    pub rect: Rect,
    pub restore_rect: Rect,
    pub open: bool,
    pub minimized: bool,
    pub maximized: bool,
}

impl Window {
    const fn new(app: AppId, rect: Rect, open: bool) -> Self {
        Self {
            app,
            rect,
            restore_rect: rect,
            open,
            minimized: false,
            maximized: false,
        }
    }
    pub fn visible(&self) -> bool {
        self.open && !self.minimized
    }
}

pub struct Desktop {
    pub width: i32,
    pub height: i32,
    pub windows: [Window; 2],
    pub focused: AppId,
    pub launcher_open: bool,
    pub launcher_selection: usize,
    pub notes: [u8; NOTE_CAPACITY],
    pub notes_len: usize,
    pub cursor: usize,
    pub notes_editing: bool,
}

impl Desktop {
    pub fn new(width: i32, height: i32) -> Self {
        let work_h = height.saturating_sub(TASKBAR_HEIGHT).max(TITLE_HEIGHT + 80);
        let about_rect = centered(width, work_h, 520, 350);
        let notes_rect = centered(width, work_h, 560, 400);
        Self {
            width,
            height,
            windows: [
                Window::new(AppId::About, about_rect, true),
                Window::new(AppId::Notes, notes_rect, false),
            ],
            focused: AppId::About,
            launcher_open: false,
            launcher_selection: 0,
            notes: [0; NOTE_CAPACITY],
            notes_len: 0,
            cursor: 0,
            notes_editing: false,
        }
    }

    pub fn window(&self, app: AppId) -> &Window {
        &self.windows[app as usize]
    }
    pub fn window_mut(&mut self, app: AppId) -> &mut Window {
        &mut self.windows[app as usize]
    }

    pub fn open(&mut self, app: AppId) {
        let w = self.window_mut(app);
        w.open = true;
        w.minimized = false;
        self.focused = app;
        self.launcher_open = false;
        if app == AppId::Notes {
            self.notes_editing = true;
        }
    }
    pub fn close(&mut self, app: AppId) {
        let w = self.window_mut(app);
        w.open = false;
        w.minimized = false;
        w.maximized = false;
        if self.focused == app {
            self.focus_next();
        }
        if app == AppId::Notes {
            self.notes_editing = false;
        }
    }
    pub fn minimize(&mut self, app: AppId) {
        if self.window(app).open {
            self.window_mut(app).minimized = true;
        }
        if self.focused == app {
            self.focus_next();
        }
    }
    pub fn restore_or_focus(&mut self, app: AppId) {
        let w = self.window_mut(app);
        if w.open {
            w.minimized = false;
            self.focused = app;
        } else {
            self.open(app);
        }
    }
    pub fn maximize_toggle(&mut self, app: AppId) {
        let width = self.width.max(1);
        let height = self
            .height
            .saturating_sub(TASKBAR_HEIGHT)
            .max(TITLE_HEIGHT + 1);
        let w = self.window_mut(app);
        if !w.open {
            return;
        }
        if w.maximized {
            w.rect = w.restore_rect;
            w.maximized = false;
        } else {
            w.restore_rect = w.rect;
            w.rect = Rect {
                x: 0,
                y: 0,
                width,
                height,
            };
            w.maximized = true;
        }
        w.minimized = false;
        self.focused = app;
    }
    pub fn focus(&mut self, app: AppId) {
        if self.window(app).visible() {
            self.focused = app;
        }
    }
    pub fn focus_next(&mut self) {
        let other = match self.focused {
            AppId::About => AppId::Notes,
            AppId::Notes => AppId::About,
        };
        if self.window(other).visible() {
            self.focused = other;
        } else if self.window(self.focused).visible() {
        } else if self.window(other).open {
            self.window_mut(other).minimized = false;
            self.focused = other;
        }
    }
    pub fn snap(&mut self, app: AppId, right: bool) {
        let screen_width = self.width;
        let width = (screen_width / 2).max(1);
        let height = self
            .height
            .saturating_sub(TASKBAR_HEIGHT)
            .max(TITLE_HEIGHT + 1);
        let w = self.window_mut(app);
        if !w.open {
            return;
        }
        w.maximized = false;
        w.minimized = false;
        w.rect = Rect {
            x: if right { screen_width - width } else { 0 },
            y: 0,
            width,
            height,
        };
        self.focused = app;
    }
    pub fn move_window(&mut self, app: AppId, dx: i32, dy: i32) {
        let max_x = (self.width - 64).max(0);
        let max_y = (self.height - TASKBAR_HEIGHT - TITLE_HEIGHT).max(0);
        let w = self.window_mut(app);
        if !w.open || w.maximized {
            return;
        }
        w.rect.x = w.rect.x.saturating_add(dx).clamp(0, max_x);
        w.rect.y = w.rect.y.saturating_add(dy).clamp(0, max_y);
    }
    pub fn resize_window(&mut self, app: AppId, dw: i32, dh: i32) {
        let screen_width = self.width;
        let screen_height = self.height;
        let max_w = (screen_width - 16).max(180);
        let max_h = (screen_height - TASKBAR_HEIGHT - 8).max(TITLE_HEIGHT + 60);
        let w = self.window_mut(app);
        if !w.open || w.maximized {
            return;
        }
        w.rect.width = w.rect.width.saturating_add(dw).clamp(180, max_w);
        w.rect.height = w
            .rect
            .height
            .saturating_add(dh)
            .clamp(TITLE_HEIGHT + 60, max_h);
        w.rect.x = w.rect.x.min((screen_width - w.rect.width).max(0));
        w.rect.y = w
            .rect
            .y
            .min((screen_height - TASKBAR_HEIGHT - TITLE_HEIGHT).max(0));
    }
    pub fn insert(&mut self, byte: u8) -> bool {
        if !self.notes_editing
            || !(byte == b'\n' || byte == b'\t' || (b' '..=b'~').contains(&byte))
            || self.notes_len >= NOTE_CAPACITY
        {
            return false;
        }
        self.notes
            .copy_within(self.cursor..self.notes_len, self.cursor + 1);
        self.notes[self.cursor] = byte;
        self.cursor += 1;
        self.notes_len += 1;
        true
    }
    pub fn backspace(&mut self) -> bool {
        if !self.notes_editing || self.cursor == 0 {
            return false;
        }
        self.notes
            .copy_within(self.cursor..self.notes_len, self.cursor - 1);
        self.cursor -= 1;
        self.notes_len -= 1;
        true
    }
    pub fn delete(&mut self) -> bool {
        if !self.notes_editing || self.cursor >= self.notes_len {
            return false;
        }
        self.notes
            .copy_within(self.cursor + 1..self.notes_len, self.cursor);
        self.notes_len -= 1;
        true
    }
    pub fn cursor_left(&mut self) {
        if self.notes_editing {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }
    pub fn cursor_right(&mut self) {
        if self.notes_editing {
            self.cursor = (self.cursor + 1).min(self.notes_len);
        }
    }
    pub fn cursor_home(&mut self) {
        if self.notes_editing {
            self.cursor = self.notes[..self.cursor]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |i| i + 1);
        }
    }
    pub fn cursor_end(&mut self) {
        if self.notes_editing {
            self.cursor = self.notes[self.cursor..self.notes_len]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(self.notes_len, |i| self.cursor + i);
        }
    }
    pub fn launcher_move(&mut self, down: bool) {
        self.launcher_selection = if down {
            (self.launcher_selection + 1) % 2
        } else {
            (self.launcher_selection + 1) % 2
        };
    }
    pub fn launcher_activate(&mut self) {
        self.open(if self.launcher_selection == 0 {
            AppId::About
        } else {
            AppId::Notes
        });
    }
}

fn centered(screen_w: i32, work_h: i32, width: i32, height: i32) -> Rect {
    let width = width.min(screen_w.max(180)).max(180);
    let height = height
        .min(work_h.max(TITLE_HEIGHT + 60))
        .max(TITLE_HEIGHT + 60);
    Rect {
        x: ((screen_w - width) / 2).max(0),
        y: ((work_h - height) / 2).max(0),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_lifecycle_and_taskbar_restoration() {
        let mut d = Desktop::new(1280, 800);
        assert!(d.window(AppId::About).visible());
        d.open(AppId::Notes);
        assert_eq!(d.focused, AppId::Notes);
        d.minimize(AppId::Notes);
        assert!(!d.window(AppId::Notes).visible());
        d.restore_or_focus(AppId::Notes);
        assert!(d.window(AppId::Notes).visible());
        d.maximize_toggle(AppId::Notes);
        assert!(d.window(AppId::Notes).maximized);
        d.maximize_toggle(AppId::Notes);
        assert!(!d.window(AppId::Notes).maximized);
        d.close(AppId::Notes);
        assert!(!d.window(AppId::Notes).open);
    }

    #[test]
    fn windows_move_resize_and_snap_within_usable_screen() {
        let mut d = Desktop::new(800, 600);
        d.move_window(AppId::About, i32::MAX, i32::MAX);
        assert!(d.window(AppId::About).rect.x <= 736);
        assert!(d.window(AppId::About).rect.y <= 538);
        d.resize_window(AppId::About, i32::MAX, i32::MAX);
        assert_eq!(d.window(AppId::About).rect.width, 784);
        assert_eq!(d.window(AppId::About).rect.height, 560);
        d.snap(AppId::About, true);
        assert_eq!(d.window(AppId::About).rect.x, 400);
    }

    #[test]
    fn notes_insert_move_backspace_and_bound_storage() {
        let mut d = Desktop::new(800, 600);
        d.open(AppId::Notes);
        assert!(d.insert(b'A'));
        assert!(d.insert(b'C'));
        d.cursor_left();
        assert!(d.insert(b'B'));
        assert_eq!(&d.notes[..d.notes_len], b"ABC");
        assert!(d.backspace());
        assert_eq!(&d.notes[..d.notes_len], b"AC");
        d.notes_editing = true;
        d.cursor = d.notes_len;
        for _ in 0..NOTE_CAPACITY {
            let _ = d.insert(b'x');
        }
        assert!(d.notes_len <= NOTE_CAPACITY);
        assert!(!d.insert(b'!'));
    }
}
