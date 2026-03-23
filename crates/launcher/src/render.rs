// Draws the launcher panel: a search input at the top, results list below.

use fontdue::{Font, FontSettings};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

use crate::apps::App;
use crate::config::{hex_to_rgba, Config};

pub struct Renderer {
    font: Option<Font>,
    pub width: u32,
    pub height: u32,
}

const ROW_H: u32 = 36;
const PAD:   f32 = 14.0;
const FONT_SIZE: f32 = 14.0;

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Renderer { font: load_font(), width, height }
    }

    pub fn render(&self, buf: &mut [u8], config: &Config, query: &str, results: &[&App], selected: usize) {
        let mut pixmap = Pixmap::new(self.width, self.height).unwrap();

        let (r, g, b, a)  = hex_to_rgba(&config.launcher.background);
        let (fr, fg, fb, _) = hex_to_rgba(&config.launcher.foreground);
        let (ar, ag, ab, _) = hex_to_rgba(&config.launcher.accent);

        // ── Panel background ──────────────────────────────────────────────
        pixmap.fill(Color::from_rgba8(r, g, b, a));

        // ── Search input row ──────────────────────────────────────────────
        // Slightly lighter than background to distinguish the input area
        let mut input_paint = Paint::default();
        input_paint.set_color(Color::from_rgba8(
            r.saturating_add(20), g.saturating_add(20), b.saturating_add(20), 255,
        ));
        if let Some(rect) = Rect::from_xywh(0.0, 0.0, self.width as f32, ROW_H as f32) {
            pixmap.fill_rect(rect, &input_paint, Transform::identity(), None);
        }

        // Accent left border on input row
        let mut accent_paint = Paint::default();
        accent_paint.set_color(Color::from_rgba8(ar, ag, ab, 255));
        if let Some(rect) = Rect::from_xywh(0.0, 0.0, 3.0, ROW_H as f32) {
            pixmap.fill_rect(rect, &accent_paint, Transform::identity(), None);
        }

        // Search query text (or placeholder)
        if let Some(font) = &self.font {
            let baseline = (ROW_H as f32 * 0.65) as i32;
            let display = if query.is_empty() { "Search apps..." } else { query };
            let (tr, tg, tb) = if query.is_empty() {
                (fr / 2, fg / 2, fb / 2) // dimmed placeholder
            } else {
                (fr, fg, fb)
            };
            draw_text(pixmap.data_mut(), self.width, self.height,
                      font, display, PAD as i32, baseline, FONT_SIZE, tr, tg, tb);
        }

        // ── Results list ──────────────────────────────────────────────────
        let max = config.launcher.max_results.min(results.len());
        for (i, app) in results.iter().take(max).enumerate() {
            let y = ROW_H + i as u32 * ROW_H;

            // Highlight selected row
            if i == selected {
                let mut sel_paint = Paint::default();
                sel_paint.set_color(Color::from_rgba8(ar, ag, ab, 40));
                if let Some(rect) = Rect::from_xywh(0.0, y as f32, self.width as f32, ROW_H as f32) {
                    pixmap.fill_rect(rect, &sel_paint, Transform::identity(), None);
                }
                // Accent bar on selected row
                let mut a_paint = Paint::default();
                a_paint.set_color(Color::from_rgba8(ar, ag, ab, 255));
                if let Some(rect) = Rect::from_xywh(0.0, y as f32, 3.0, ROW_H as f32) {
                    pixmap.fill_rect(rect, &a_paint, Transform::identity(), None);
                }
            }

            if let Some(font) = &self.font {
                let baseline = y as i32 + (ROW_H as f32 * 0.65) as i32;
                draw_text(pixmap.data_mut(), self.width, self.height,
                          font, &app.name, PAD as i32, baseline, FONT_SIZE, fr, fg, fb);
            }
        }

        // ── Copy RGBA → ARGB8888 ──────────────────────────────────────────
        for (d, s) in buf.chunks_exact_mut(4).zip(pixmap.data().chunks_exact(4)) {
            d[0] = s[2]; d[1] = s[1]; d[2] = s[0]; d[3] = s[3];
        }
    }
}

// ── Text rendering ────────────────────────────────────────────────────────────

fn draw_text(data: &mut [u8], w: u32, h: u32, font: &Font, text: &str,
             x: i32, baseline: i32, size: f32, r: u8, g: u8, b: u8) {
    let mut cx = x;
    for ch in text.chars() {
        let (m, bitmap) = font.rasterize(ch, size);
        if m.width == 0 { cx += m.advance_width as i32; continue; }
        let gx = cx + m.xmin;
        let gy = baseline - m.ymin - m.height as i32;
        for (i, &cov) in bitmap.iter().enumerate() {
            if cov == 0 { continue; }
            let px = gx + (i % m.width) as i32;
            let py = gy + (i / m.width) as i32;
            if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 { continue; }
            let idx = ((py * w as i32 + px) * 4) as usize;
            let a = cov as u32; let ia = 255 - a;
            data[idx]     = ((r as u32*a + data[idx]    as u32*ia)/255) as u8;
            data[idx+1]   = ((g as u32*a + data[idx+1]  as u32*ia)/255) as u8;
            data[idx+2]   = ((b as u32*a + data[idx+2]  as u32*ia)/255) as u8;
            data[idx+3]   = 255;
        }
        cx += m.advance_width as i32;
    }
}

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
