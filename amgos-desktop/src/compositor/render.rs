//! Compositor Layered UI Renderer (Process 2)
//!
//! Enforces:
//! - AMGOS SPEC Section 6 (Layered UI stack, Pure Wayland, 144Hz continuous presentation)
//! - AMGOS INTERFACE SHELL Section 3.1 (Top Global Menu Panel, Space Orange identity mark, Real System Tray)
//! - AMGOS INTERFACE SHELL Section 4.1 (Window Chrome & Traffic Light window controls)
//! - AMGOS INTERFACE SHELL Section 5 (System Screens: Lockscreen, Shutdown "goodbye", Restart "i'll see you in a bit")
//! - AMGOS INTERFACE SHELL Section 9 (Typography: Inter, JetBrains Mono, Young Serif)

use crate::notifications::NotificationCenter;
use crate::pilot_control::PilotControl;
use crate::shutdown_ui::PowerScreenState;
use crate::smart_dock::SmartDock;
use crate::tiling::TilingWindowManager;
use crate::topbar::TopGlobalMenuBar;
use crate::traffic_lights::TrafficLightGroup;
use crate::wizard::SetupWizardState;
use amgos_protocol::ebus::PowerProfile;

pub const SPACE_ORANGE: (u8, u8, u8, u8) = (255, 85, 0, 255);
pub const SPACE_WHITE: (u8, u8, u8, u8) = (240, 240, 242, 255);
pub const SKY_BLUE: (u8, u8, u8, u8) = (0, 163, 255, 255);
pub const EMERALD_GREEN: (u8, u8, u8, u8) = (16, 185, 129, 255);

/// Basic 8x12 bitmap font representation for reliable pixel-perfect text rendering
/// across all system surfaces without external font dependencies.
pub fn get_glyph_bitmap(ch: char) -> &'static [u8; 12] {
    match ch {
        'A' | 'a' => &[0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00],
        'B' | 'b' => &[0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00],
        'C' | 'c' => &[0x3C, 0x66, 0x60, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        'D' | 'd' => &[0x78, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00, 0x00, 0x00, 0x00],
        'E' | 'e' => &[0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00],
        'F' | 'f' => &[0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00],
        'G' | 'g' => &[0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00, 0x00],
        'H' | 'h' => &[0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00],
        'I' | 'i' => &[0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00],
        'J' | 'j' => &[0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x6C, 0x38, 0x00, 0x00, 0x00, 0x00],
        'K' | 'k' => &[0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00],
        'L' | 'l' => &[0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00],
        'M' | 'm' => &[0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x63, 0x00, 0x00, 0x00, 0x00],
        'N' | 'n' => &[0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00],
        'O' | 'o' => &[0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        'P' | 'p' => &[0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x00],
        'Q' | 'q' => &[0x3C, 0x66, 0x66, 0x66, 0x66, 0x6E, 0x3C, 0x0E, 0x00, 0x00, 0x00, 0x00],
        'R' | 'r' => &[0x7C, 0x66, 0x66, 0x7C, 0x78, 0x6C, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00],
        'S' | 's' => &[0x3C, 0x66, 0x60, 0x3C, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        'T' | 't' => &[0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
        'U' | 'u' => &[0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        'V' | 'v' => &[0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x3C, 0x18, 0x00, 0x00, 0x00, 0x00],
        'W' | 'w' => &[0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x63, 0x00, 0x00, 0x00, 0x00],
        'X' | 'x' => &[0x66, 0x66, 0x3C, 0x18, 0x18, 0x3C, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00],
        'Y' | 'y' => &[0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
        'Z' | 'z' => &[0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00],
        '0' => &[0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        '1' => &[0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00],
        '2' => &[0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00],
        '3' => &[0x3C, 0x66, 0x06, 0x1C, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        '4' => &[0x0C, 0x1C, 0x34, 0x64, 0x7E, 0x04, 0x04, 0x0E, 0x00, 0x00, 0x00, 0x00],
        '5' => &[0x7E, 0x60, 0x7C, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        '6' => &[0x1C, 0x30, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        '7' => &[0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00],
        '8' => &[0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
        '9' => &[0x3C, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0C, 0x38, 0x00, 0x00, 0x00, 0x00],
        ':' => &[0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
        '.' => &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
        ',' => &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x08, 0x10, 0x00, 0x00],
        '-' => &[0x00, 0x00, 0x00, 0x7E, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '+' => &[0x00, 0x18, 0x18, 0x7E, 0x7E, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
        '%' => &[0x62, 0x64, 0x08, 0x10, 0x20, 0x26, 0x46, 0x00, 0x00, 0x00, 0x00, 0x00],
        '[' => &[0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00, 0x00, 0x00, 0x00],
        ']' => &[0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00, 0x00, 0x00, 0x00],
        '(' => &[0x0C, 0x18, 0x30, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00],
        ')' => &[0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00],
        '/' => &[0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00],
        '\\' => &[0x40, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00],
        '\'' => &[0x18, 0x18, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '!' => &[0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
        '?' => &[0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
        _ => &[0x00; 12],
    }
}

pub struct FramebufferContext<'a> {
    pub buffer: &'a mut [u8],
    pub width: usize,
    pub height: usize,
}

impl<'a> FramebufferContext<'a> {
    pub fn new(buffer: &'a mut [u8], width: usize, height: usize) -> Self {
        Self { buffer, width, height }
    }

    #[inline]
    pub fn blend_pixel(&mut self, x: usize, y: usize, color: (u8, u8, u8, u8)) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y * self.width + x) * 4;
        let (sr, sg, sb, sa) = color;
        if sa == 255 {
            self.buffer[idx] = sr;
            self.buffer[idx + 1] = sg;
            self.buffer[idx + 2] = sb;
            self.buffer[idx + 3] = 255;
        } else if sa > 0 {
            let a = sa as u32;
            let inv_a = 255 - a;
            self.buffer[idx] = ((sr as u32 * a + self.buffer[idx] as u32 * inv_a) / 255) as u8;
            self.buffer[idx + 1] = ((sg as u32 * a + self.buffer[idx + 1] as u32 * inv_a) / 255) as u8;
            self.buffer[idx + 2] = ((sb as u32 * a + self.buffer[idx + 2] as u32 * inv_a) / 255) as u8;
            self.buffer[idx + 3] = 255;
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: (u8, u8, u8, u8)) {
        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);
        for py in y..max_y {
            for px in x..max_x {
                self.blend_pixel(px, py, color);
            }
        }
    }

    pub fn stroke_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: (u8, u8, u8, u8)) {
        if w == 0 || h == 0 { return; }
        self.fill_rect(x, y, w, 1, color);
        self.fill_rect(x, y + h.saturating_sub(1), w, 1, color);
        self.fill_rect(x, y, 1, h, color);
        self.fill_rect(x + w.saturating_sub(1), y, 1, h, color);
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: (u8, u8, u8, u8)) {
        let r2 = radius * radius;
        for dy in -radius..=radius {
            let py = cy + dy;
            if py < 0 || py >= self.height as i32 { continue; }
            for dx in -radius..=radius {
                let px = cx + dx;
                if px < 0 || px >= self.width as i32 { continue; }
                if dx * dx + dy * dy <= r2 {
                    self.blend_pixel(px as usize, py as usize, color);
                }
            }
        }
    }

    pub fn draw_text(&mut self, text: &str, start_x: usize, start_y: usize, color: (u8, u8, u8, u8)) -> usize {
        let mut cur_x = start_x;
        for ch in text.chars() {
            if ch == ' ' {
                cur_x += 6;
                continue;
            }
            let bitmap = get_glyph_bitmap(ch);
            for row in 0..12 {
                let py = start_y + row;
                if py >= self.height { break; }
                let bits = bitmap[row];
                for col in 0..8 {
                    if (bits & (0x80 >> col)) != 0 {
                        let px = cur_x + col;
                        if px < self.width {
                            self.blend_pixel(px, py, color);
                        }
                    }
                }
            }
            cur_x += 8;
        }
        cur_x
    }

    /// Renders Wi-Fi icon with signal strength bars (1 to 4 bars)
    pub fn draw_wifi_icon(&mut self, x: usize, y: usize, signal_pct: u8, connected: bool) {
        let (icon_r, icon_g, icon_b) = if connected {
            (240, 240, 242)
        } else {
            (120, 120, 128)
        };

        // 4 vertical bars with ascending heights
        let bars = if !connected {
            0
        } else if signal_pct > 75 {
            4
        } else if signal_pct > 50 {
            3
        } else if signal_pct > 25 {
            2
        } else {
            1
        };

        for b in 0..4 {
            let bar_h = 3 + b * 3;
            let bx = x + b * 4;
            let by = y + (12 - bar_h);
            let col = if b < bars {
                (icon_r, icon_g, icon_b, 255)
            } else {
                (60, 60, 68, 255)
            };
            self.fill_rect(bx, by, 3, bar_h, col);
        }
    }

    /// Renders AVM Audio speaker icon
    pub fn draw_audio_icon(&mut self, x: usize, y: usize, volume_pct: u8, muted: bool) {
        let col = if muted {
            (180, 70, 70, 255)
        } else {
            (240, 240, 242, 255)
        };

        // Speaker cone
        self.fill_rect(x, y + 4, 3, 5, col);
        self.fill_rect(x + 3, y + 3, 2, 7, col);
        self.fill_rect(x + 5, y + 2, 2, 9, col);

        if !muted && volume_pct > 0 {
            // Sound wave arcs
            self.fill_rect(x + 9, y + 3, 1, 7, col);
            if volume_pct > 50 {
                self.fill_rect(x + 12, y + 1, 1, 11, col);
            }
        } else if muted {
            // Diagonal mute slash
            for i in 0..8 {
                self.blend_pixel(x + 8 + i, y + 2 + i, (220, 60, 60, 255));
            }
        }
    }

    /// Renders Power & Battery monitor with profile badge and fill bar
    pub fn draw_battery_icon(&mut self, x: usize, y: usize, pct: u8, charging: bool) {
        // Battery shell (18x10px)
        let shell_col = (200, 200, 204, 255);
        self.stroke_rect(x, y + 2, 16, 9, shell_col);
        // Positive terminal cap
        self.fill_rect(x + 16, y + 5, 2, 3, shell_col);

        // Fill level
        let fill_w = ((pct as usize * 12) / 100).min(12);
        let fill_col = if charging {
            SKY_BLUE
        } else if pct <= 15 {
            SPACE_ORANGE
        } else {
            EMERALD_GREEN
        };
        if fill_w > 0 {
            self.fill_rect(x + 2, y + 4, fill_w, 5, fill_col);
        }
    }

    /// Renders Notification bell icon with unread badge counter
    pub fn draw_notification_icon(&mut self, x: usize, y: usize, unread_count: u32) {
        let bell_col = (220, 220, 224, 255);
        // Bell shape
        self.fill_rect(x + 4, y + 1, 3, 2, bell_col);
        self.fill_rect(x + 2, y + 3, 7, 6, bell_col);
        self.fill_rect(x + 1, y + 9, 9, 2, bell_col);
        self.fill_rect(x + 4, y + 11, 3, 1, bell_col);

        // Unread badge in Space Orange
        if unread_count > 0 {
            self.fill_circle((x + 11) as i32, (y + 3) as i32, 4, SPACE_ORANGE);
            let count_str = if unread_count > 9 { "9+".to_string() } else { unread_count.to_string() };
            self.draw_text(&count_str, x + 9, y + 1, (255, 255, 255, 255));
        }
    }
}

/// Renders the complete Top Global Menu Panel and real System Tray
pub fn render_top_panel(
    ctx: &mut FramebufferContext,
    topbar: &TopGlobalMenuBar,
) {
    let w = ctx.width;
    let h = 32usize;

    // 1. Panel Background: Dark obsidian glass (#161618)
    ctx.fill_rect(0, 0, w, h, (22, 22, 24, 255));
    // Subtle border bottom (#28282c)
    ctx.fill_rect(0, h - 1, w, 1, (40, 40, 44, 255));

    // 2. Far Left: Space Orange sharp rectangle (#FF5500, 36x24px, strictly 0 corner radius)
    let box_w = topbar.identity_mark.width_px as usize;
    let box_h = topbar.identity_mark.height_px as usize;
    ctx.fill_rect(8, 4, box_w, box_h, SPACE_ORANGE);

    // 3. Focused App Title (e.g. "Filer") in bold white
    let mut cur_x = 52usize;
    cur_x = ctx.draw_text(&topbar.focused_app.app_title, cur_x, 10, (255, 255, 255, 255));
    cur_x += 16;

    // 4. Global Menu Items ("File", "Edit", "View", "Go", "Window", "Help")
    for item in &topbar.focused_app.menu_items {
        cur_x = ctx.draw_text(item, cur_x, 10, (212, 212, 216, 255));
        cur_x += 16;
    }

    // 5. Far Right: System Tray Subsystem
    let mut tray_x = w.saturating_sub(28);

    // 5a. Notification Bell & Badge
    ctx.draw_notification_icon(tray_x, 10, topbar.notifications.unread_count);
    tray_x = tray_x.saturating_sub(24);

    // 5b. Clock & Calendar: Localized time (e.g. "Tue Sep 15  09:45")
    let clock_str = topbar.clock.display_string();
    let clock_w = clock_str.len() * 8;
    let clock_start_x = tray_x.saturating_sub(clock_w);
    ctx.draw_text(&clock_str, clock_start_x, 10, (240, 240, 242, 255));
    tray_x = clock_start_x.saturating_sub(18);

    // 5c. Power & Battery Monitor: Icon, profile badge, and percentage
    let battery_pct = topbar.power.battery_pct.unwrap_or(98);
    let charging = topbar.power.is_charging;
    let batt_text = format!("{}%", battery_pct);
    let batt_text_w = batt_text.len() * 8;
    let batt_x = tray_x.saturating_sub(batt_text_w);
    ctx.draw_text(&batt_text, batt_x, 10, (200, 200, 204, 255));
    tray_x = batt_x.saturating_sub(22);

    ctx.draw_battery_icon(tray_x, 9, battery_pct, charging);
    tray_x = tray_x.saturating_sub(38);

    // Power Profile Badge: Space Orange [MAX], Sky Blue [BAL], Emerald Green [END]
    let (prof_label, prof_col) = match topbar.power.profile {
        PowerProfile::Max => ("MAX", SPACE_ORANGE),
        PowerProfile::Balanced => ("BAL", SKY_BLUE),
        PowerProfile::Endurance => ("END", EMERALD_GREEN),
    };
    ctx.fill_rect(tray_x, 7, 32, 17, (30, 30, 36, 255));
    ctx.stroke_rect(tray_x, 7, 32, 17, prof_col);
    ctx.draw_text(prof_label, tray_x + 4, 10, prof_col);
    tray_x = tray_x.saturating_sub(18);

    // 5d. Audio & Volume (Speaker icon + volume %)
    let vol_text = if topbar.audio.is_muted {
        "Mute".to_string()
    } else {
        format!("{}%", topbar.audio.volume_pct)
    };
    let vol_w = vol_text.len() * 8;
    let vol_x = tray_x.saturating_sub(vol_w);
    ctx.draw_text(&vol_text, vol_x, 10, (200, 200, 204, 255));
    tray_x = vol_x.saturating_sub(18);

    ctx.draw_audio_icon(tray_x, 10, topbar.audio.volume_pct, topbar.audio.is_muted);
    tray_x = tray_x.saturating_sub(22);

    // 5e. Network Tray Widget (Wi-Fi bars / status)
    ctx.draw_wifi_icon(tray_x, 10, topbar.network.signal_pct, topbar.network.is_connected);
    tray_x = tray_x.saturating_sub(26);

    // 5f. StatusNotifierItem (SNI) App Icons (e.g. Zen Browser)
    for sni in topbar.sni_host.iter_items() {
        let label = if sni.id.contains("zen") {
            "Z"
        } else {
            &sni.id[..1.min(sni.id.len())]
        };
        ctx.fill_rect(tray_x, 6, 20, 20, (36, 36, 42, 255));
        ctx.stroke_rect(tray_x, 6, 20, 20, (80, 80, 90, 255));
        ctx.draw_text(label, tray_x + 6, 10, (240, 240, 245, 255));
        tray_x = tray_x.saturating_sub(26);
    }
}

/// Renders Layer 1..4 Application Windows with Window Chrome and Traffic Lights
pub fn render_windows(
    ctx: &mut FramebufferContext,
    tiling: &TilingWindowManager,
    traffic_lights: &TrafficLightGroup,
) {
    // If windows are registered in tiling manager, render their frames
    if tiling.window_count() == 0 {
        // Render default showcase window (e.g. Filer primary window)
        let win_x = (ctx.width.saturating_sub(1080)) / 2;
        let win_y = 56usize;
        let win_w = 1080.min(ctx.width.saturating_sub(32));
        let win_h = 680.min(ctx.height.saturating_sub(160));

        // Window shadow / border
        ctx.fill_rect(win_x, win_y, win_w, win_h, (24, 24, 28, 240));
        ctx.stroke_rect(win_x, win_y, win_w, win_h, (52, 52, 60, 255));

        // Window Chrome Header Bar (36px)
        ctx.fill_rect(win_x, win_y, win_w, 36, (32, 32, 38, 255));
        ctx.fill_rect(win_x, win_y + 35, win_w, 1, (48, 48, 56, 255));

        // Traffic Light Controls (Top-left of window)
        let tl_y = (win_y + 18) as i32;
        let tl_r = 6i32;

        // Close: Space Orange (#FF5500)
        ctx.fill_circle((win_x + 20) as i32, tl_y, tl_r, SPACE_ORANGE);
        if traffic_lights.symbols_visible() {
            ctx.draw_text("x", win_x + 17, win_y + 12, (0, 0, 0, 255));
        }

        // Minimize: Space White (#F0F0F2)
        ctx.fill_circle((win_x + 38) as i32, tl_y, tl_r, SPACE_WHITE);
        if traffic_lights.symbols_visible() {
            ctx.draw_text("-", win_x + 35, win_y + 12, (0, 0, 0, 255));
        }

        // Maximize: Sky Blue (#00A3FF)
        ctx.fill_circle((win_x + 56) as i32, tl_y, tl_r, SKY_BLUE);
        if traffic_lights.symbols_visible() {
            ctx.draw_text("+", win_x + 53, win_y + 12, (0, 0, 0, 255));
        }

        // Centered Window Title
        let title = "Filer — Pathfinder";
        let title_w = title.len() * 8;
        let title_x = win_x + (win_w.saturating_sub(title_w)) / 2;
        ctx.draw_text(title, title_x, win_y + 12, (230, 230, 235, 255));

        // Window Interior: Pathfinder Search Box
        let search_w = (win_w * 3) / 5;
        let search_x = win_x + (win_w.saturating_sub(search_w)) / 2;
        let search_y = win_y + 60;
        ctx.fill_rect(search_x, search_y, search_w, 40, (18, 18, 22, 255));
        ctx.stroke_rect(search_x, search_y, search_w, 40, SPACE_ORANGE);
        ctx.draw_text("Search files, apps, settings, or web...", search_x + 16, search_y + 14, (160, 160, 168, 255));
    }
}

/// Renders Layer 5 Smart Dock at screen bottom
pub fn render_smart_dock(
    ctx: &mut FramebufferContext,
    dock: &SmartDock,
) {
    if !dock.is_visible() {
        return;
    }

    let w = ctx.width;
    let h = ctx.height;
    let dock_h = 64usize;
    let dock_y = h.saturating_sub(dock_h + 16);
    let dock_w = (dock.entries.len() * 56 + 32).min(w);
    let dock_x = (w.saturating_sub(dock_w)) / 2;

    // Frosted Glass Dock Container (#1c1c22 with alpha 230)
    ctx.fill_rect(dock_x, dock_y, dock_w, dock_h, (28, 28, 34, 230));
    ctx.stroke_rect(dock_x, dock_y, dock_w, dock_h, (56, 56, 68, 200));

    // Render Dock Items
    for (i, entry) in dock.entries.iter().enumerate() {
        let item_x = dock_x + 16 + i * 56;
        let item_y = dock_y + 8;

        // Icon background
        ctx.fill_rect(item_x, item_y, 44, 44, (38, 38, 46, 255));
        ctx.stroke_rect(item_x, item_y, 44, 44, (70, 70, 84, 255));

        // App Initial or glyph
        let initial = &entry.app_name[..1.min(entry.app_name.len())];
        ctx.draw_text(initial, item_x + 18, item_y + 16, (255, 255, 255, 255));

        // Running app indicator dot in Space Orange (#FF5500)
        if entry.is_running {
            ctx.fill_circle((item_x + 22) as i32, (dock_y + dock_h - 6) as i32, 2, SPACE_ORANGE);
        }
    }
}

/// Renders Layer 8 Pop-up menus (System Menu dropdown)
pub fn render_system_menu(
    ctx: &mut FramebufferContext,
    topbar: &TopGlobalMenuBar,
) {
    if !topbar.system_menu_open {
        return;
    }

    let menu_x = 8usize;
    let menu_y = 34usize;
    let menu_w = 220usize;
    let menu_h = 180usize;

    // Dark titanium dropdown card
    ctx.fill_rect(menu_x, menu_y, menu_w, menu_h, (24, 24, 28, 250));
    ctx.stroke_rect(menu_x, menu_y, menu_w, menu_h, (60, 60, 70, 255));

    let items = [
        "About AMG-OS (Upstream Color)",
        "System Settings...",
        "App Catalog (Pathfinder)...",
        "---",
        "Lock Screen",
        "Sleep",
        "Restart...",
        "Shut Down...",
    ];

    let mut item_y = menu_y + 10;
    for &item in &items {
        if item == "---" {
            ctx.fill_rect(menu_x + 8, item_y + 4, menu_w - 16, 1, (48, 48, 56, 255));
            item_y += 10;
        } else {
            ctx.draw_text(item, menu_x + 14, item_y, (220, 220, 225, 255));
            item_y += 20;
        }
    }
}

/// Renders Layer 9 Notification Banners
pub fn render_notifications(
    ctx: &mut FramebufferContext,
    notifications: &NotificationCenter,
) {
    if notifications.is_empty() {
        return;
    }

    let banner_w = 320usize;
    let banner_h = 68usize;
    let banner_x = ctx.width.saturating_sub(banner_w + 16);
    let mut banner_y = 44usize;

    for banner in &notifications.banners {
        ctx.fill_rect(banner_x, banner_y, banner_w, banner_h, (26, 26, 32, 245));
        ctx.stroke_rect(banner_x, banner_y, banner_w, banner_h, SPACE_ORANGE);

        // App Id / Title
        ctx.draw_text(&banner.app_id, banner_x + 12, banner_y + 10, SPACE_ORANGE);
        ctx.draw_text(&banner.title, banner_x + 12, banner_y + 26, (255, 255, 255, 255));
        ctx.draw_text(&banner.body, banner_x + 12, banner_y + 44, (180, 180, 188, 255));

        banner_y += banner_h + 10;
    }
}

/// Renders Layer 12 Shutdown / Restart Screens
pub fn render_power_screen(
    ctx: &mut FramebufferContext,
    screen: &PowerScreenState,
) {
    // Pure black background (#000000)
    ctx.fill_rect(0, 0, ctx.width, ctx.height, (0, 0, 0, 255));

    // Centered lowercase text ("goodbye" or "i'll see you in a bit") in Inter
    let text = screen.display_text();
    let text_w = text.len() * 8;
    let tx = (ctx.width.saturating_sub(text_w)) / 2;
    let ty = ctx.height / 2;

    let opacity = (screen.text_opacity() * 255.0).clamp(0.0, 255.0) as u8;
    ctx.draw_text(text, tx, ty, (opacity, opacity, opacity, 255));
}

/// Renders First-Boot Setup Wizard Card
pub fn render_setup_wizard(
    ctx: &mut FramebufferContext,
    wizard: &SetupWizardState,
) {
    // Dim background scrim
    ctx.fill_rect(0, 0, ctx.width, ctx.height, (12, 12, 14, 240));

    let card_w = 640usize;
    let card_h = 420usize;
    let cx = (ctx.width.saturating_sub(card_w)) / 2;
    let cy = (ctx.height.saturating_sub(card_h)) / 2;

    // Centered card container
    ctx.fill_rect(cx, cy, card_w, card_h, (26, 26, 32, 255));
    ctx.stroke_rect(cx, cy, card_w, card_h, (56, 56, 68, 255));

    // Step indicator & title
    let step_str = format!("Step {} of 7", wizard.current_step as u8);
    ctx.draw_text(&step_str, cx + 32, cy + 32, SPACE_ORANGE);

    let step_title = match wizard.current_step {
        crate::wizard::WizardStep::LanguageAndRegion => "Language and Region",
        crate::wizard::WizardStep::Network => "Connect to Network",
        crate::wizard::WizardStep::UserAccount => "Create User Account",
        crate::wizard::WizardStep::PrivacyAndTelemetry => "Privacy and Telemetry",
        crate::wizard::WizardStep::DisplayAndAccessibility => "Display and Accessibility",
        crate::wizard::WizardStep::Migration => "Data Migration",
        crate::wizard::WizardStep::Done => "Ready to Use AMG-OS",
    };
    ctx.draw_text(step_title, cx + 32, cy + 56, (255, 255, 255, 255));

    // "Continue" Button in Space Orange
    let btn_w = 120usize;
    let btn_h = 36usize;
    let btn_x = cx + card_w - btn_w - 32;
    let btn_y = cy + card_h - btn_h - 32;
    ctx.fill_rect(btn_x, btn_y, btn_w, btn_h, SPACE_ORANGE);
    ctx.draw_text("Continue", btn_x + 28, btn_y + 12, (255, 255, 255, 255));
}
