// Renders the dock: a solid background bar with colored app icon slots.
// Icons are rounded rectangles with the app's initial letter centered inside.

use fontdue::{Font, FontSettings};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

use crate::config::{hex_to_rgba, Config};

// Pinned apps — exec is the shell command to launch.
// Colors give each app a distinct identity even before real icons are added.
pub struct App {
    pub name: &'static str,
    pub exec: &'static str,
    pub color: (u8, u8, u8),
}

pub const PINNED_APPS: &[App] = &[
    App { name: "Terminal", exec: "foot",    color: (76,  175, 80)  },
    App { name: "Browser",  exec: "firefox", color: (255, 152, 0)   },
    App { name: "Editor",   exec: "vim",     color: (33,  150, 243) },
    App { name: "Files",    exec: "pcmanfm", color: (156, 39,  176) },
    App { name: "Launcher", exec: "pepos-launcher", color: (94, 155, 255) },
];

pub struct Renderer {
    font: Option<Font>,
    pub width: u32,
    pub height: u32,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        let font = load_font();
        Renderer { font, width, height }
    }

    pub fn render(&self, buf: &mut [u8], config: &Config) {
        let mut pixmap = Pixmap::new(self.width, self.height).unwrap();

        // ── Background ────────────────────────────────────────────────────
        let (r, g, b, a) = hex_to_rgba(&config.dock.background);
        pixmap.fill(Color::from_rgba8(r, g, b, a));

        // ── App icons ─────────────────────────────────────────────────────
        let icon = config.dock.icon_size as f32;
        let pad  = config.dock.padding as f32;
        let gap  = 10.0_f32;
        let n    = PINNED_APPS.len() as f32;

        // Center the icon row horizontally
        let row_w   = n * icon + (n - 1.0) * gap;
        let start_x = (self.width as f32 - row_w) / 2.0;
        let icon_y  = pad;

        let (fr, fg, fb, _) = hex_to_rgba(&config.dock.foreground);

        for (i, app) in PINNED_APPS.iter().enumerate() {
            let x = start_x + i as f32 * (icon + gap);

            // Rounded rectangle icon background
            let (ir, ig, ib) = app.color;
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(ir, ig, ib, 255));
            paint.anti_alias = true;

            if let Some(path) = rounded_rect(x, icon_y, icon, icon, config.dock.corner_radius) {
                pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
            }

            // Initial letter centered in the icon
            if let Some(font) = &self.font {
                let ch = app.name.chars().next().unwrap_or('?')
                    .to_uppercase().next().unwrap_or('?');
                let size = icon * 0.42;
                let (m, bitmap) = font.rasterize(ch, size);

                // Center glyph inside the icon rect
                let cx = x as i32 + (icon as i32 - m.width as i32) / 2 + m.xmin;
                let cy_baseline = icon_y as i32 + (icon as i32 + m.height as i32) / 2 - m.ymin;

                blend_glyph(pixmap.data_mut(), self.width, self.height,
                            &bitmap, m.width, m.height,
                            cx, cy_baseline - m.height as i32,
                            fr, fg, fb);
            }
        }

        // ── Copy RGBA → Wayland ARGB8888 ─────────────────────────────────
        for (d, s) in buf.chunks_exact_mut(4).zip(pixmap.data().chunks_exact(4)) {
            d[0] = s[2]; d[1] = s[1]; d[2] = s[0]; d[3] = s[3];
        }
    }
}

// ── Rounded rectangle path ────────────────────────────────────────────────────

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y,     x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x,     y + h, x,     y + h - r);
    pb.line_to(x,     y + r);
    pb.quad_to(x,     y,     x + r, y);
    pb.close();
    pb.finish()
}

// ── Glyph blending ────────────────────────────────────────────────────────────

fn blend_glyph(
    data: &mut [u8],
    width: u32, height: u32,
    bitmap: &[u8],
    gw: usize, gh: usize,
    x: i32, y: i32,
    r: u8, g: u8, b: u8,
) {
    for row in 0..gh {
        for col in 0..gw {
            let cov = bitmap[row * gw + col];
            if cov == 0 { continue; }
            let px = x + col as i32;
            let py = y + row as i32;
            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 { continue; }
            let idx = ((py * width as i32 + px) * 4) as usize;
            let a = cov as u32;
            let ia = 255 - a;
            data[idx]     = ((r as u32 * a + data[idx]     as u32 * ia) / 255) as u8;
            data[idx + 1] = ((g as u32 * a + data[idx + 1] as u32 * ia) / 255) as u8;
            data[idx + 2] = ((b as u32 * a + data[idx + 2] as u32 * ia) / 255) as u8;
            data[idx + 3] = 255;
        }
    }
}

// ── Font loading ──────────────────────────────────────────────────────────────

fn load_font() -> Option<Font> {
    let dirs = ["/usr/share/fonts/TTF", "/usr/share/fonts",
                "/usr/share/fonts/truetype", "/usr/local/share/fonts"];
    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "ttf").unwrap_or(false) {
                    if let Ok(bytes) = std::fs::read(&p) {
                        if let Ok(font) = Font::from_bytes(bytes.as_slice(), FontSettings::default()) {
                            return Some(font);
                        }
                    }
                }
            }
        }
    }
    None
}
