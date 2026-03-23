// Renders the menubar frame into a raw pixel buffer.
//
// Pipeline:
//   1. Fill background with solid color (tiny-skia)
//   2. Rasterize text glyphs (fontdue)
//   3. Alpha-blend glyphs onto the pixmap
//   4. Convert tiny-skia RGBA → Wayland ARGB8888 and write to the shm buffer

use fontdue::{Font, FontSettings};
use tiny_skia::{Color, Pixmap};

use crate::config::{hex_to_rgba, Config};

pub struct Renderer {
    font: Option<Font>,
    pub width: u32,
    pub height: u32,
}

impl Renderer {
    pub fn new(width: u32, height: u32, config: &Config) -> Self {
        let font = load_font(&config.menubar.font);
        if font.is_none() {
            tracing::warn!(
                "no font found — install a TTF font package on Void Linux. \
                 Text will not render until a font is available."
            );
        }
        Renderer { font, width, height }
    }

    /// Draw a complete frame and write it into `buf` (the shm mmap slice).
    pub fn render(&self, buf: &mut [u8], config: &Config) {
        let mut pixmap = Pixmap::new(self.width, self.height)
            .expect("failed to create pixmap");

        // ── Background ───────────────────────────────────────────────────────
        let (r, g, b, a) = hex_to_rgba(&config.menubar.background);
        pixmap.fill(Color::from_rgba8(r, g, b, a));

        // ── Text ─────────────────────────────────────────────────────────────
        if let Some(font) = &self.font {
            let size = config.menubar.font_size;
            let (fr, fg, fb, _) = hex_to_rgba(&config.menubar.foreground);

            // Baseline: visually center caps in the bar height.
            // 0.72 * font_size ≈ cap height; place it centered vertically.
            let baseline = ((self.height as f32 + size * 0.72) / 2.0) as i32;

            // Left: distro label
            draw_text(pixmap.data_mut(), self.width, self.height,
                      font, "pepos", 14, baseline, size, fr, fg, fb);

            // Right: clock
            let time = chrono::Local::now().format("%H:%M").to_string();
            let time_w = measure_text(font, &time, size);
            let x = self.width as i32 - time_w as i32 - 14;
            draw_text(pixmap.data_mut(), self.width, self.height,
                      font, &time, x, baseline, size, fr, fg, fb);
        }

        // ── Copy to Wayland buffer ────────────────────────────────────────────
        // tiny-skia pixel layout: [R, G, B, A] (premultiplied)
        // Wayland ARGB8888 on little-endian x86: bytes are [B, G, R, A]
        for (dst, src) in buf.chunks_exact_mut(4).zip(pixmap.data().chunks_exact(4)) {
            dst[0] = src[2]; // B
            dst[1] = src[1]; // G
            dst[2] = src[0]; // R
            dst[3] = src[3]; // A
        }
    }
}

// ── Font loading ──────────────────────────────────────────────────────────────

fn load_font(preferred_name: &str) -> Option<Font> {
    // Common font directories on Void Linux
    let dirs = [
        "/usr/share/fonts/TTF",
        "/usr/share/fonts",
        "/usr/share/fonts/truetype",
        "/usr/local/share/fonts",
    ];

    // Try preferred font by common filename patterns first
    for dir in &dirs {
        for suffix in &["Regular", "regular", ""] {
            let sep = if suffix.is_empty() { "" } else { "-" };
            let path = format!("{}/{}{}{}.ttf", dir, preferred_name, sep, suffix);
            if let Some(font) = try_load_font(&path) {
                tracing::info!("loaded font: {}", path);
                return Some(font);
            }
        }
    }

    // Fall back to any .ttf file we can find
    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "ttf").unwrap_or(false) {
                    if let Some(font) = try_load_font(p.to_str().unwrap_or("")) {
                        tracing::info!("loaded fallback font: {:?}", p);
                        return Some(font);
                    }
                }
            }
        }
    }

    None
}

fn try_load_font(path: &str) -> Option<Font> {
    let bytes = std::fs::read(path).ok()?;
    Font::from_bytes(bytes.as_slice(), FontSettings::default()).ok()
}

// ── Text rendering ────────────────────────────────────────────────────────────

/// Returns the advance width of a string in pixels at the given size.
fn measure_text(font: &Font, text: &str, size: f32) -> f32 {
    text.chars().map(|c| font.rasterize(c, size).0.advance_width).sum()
}

/// Rasterize `text` and alpha-blend it onto `data` (tiny-skia RGBA pixel buffer).
fn draw_text(
    data: &mut [u8],
    width: u32,
    height: u32,
    font: &Font,
    text: &str,
    start_x: i32,
    baseline_y: i32,
    size: f32,
    r: u8,
    g: u8,
    b: u8,
) {
    let mut cx = start_x;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        if metrics.width == 0 || metrics.height == 0 {
            cx += metrics.advance_width as i32;
            continue;
        }

        // Convert fontdue's y-up coordinates to our y-down screen coordinates.
        // metrics.ymin = distance from baseline to bottom of glyph bbox (y-up).
        let glyph_top = baseline_y - metrics.ymin - metrics.height as i32;
        let glyph_left = cx + metrics.xmin;

        for (i, &coverage) in bitmap.iter().enumerate() {
            if coverage == 0 {
                continue;
            }
            let px = glyph_left + (i % metrics.width) as i32;
            let py = glyph_top + (i / metrics.width) as i32;

            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                continue;
            }

            let idx = ((py * width as i32 + px) * 4) as usize;

            // Alpha-blend: out = glyph_color * coverage + bg * (1 - coverage)
            let a = coverage as u32;
            let ia = 255 - a;
            data[idx]     = ((r as u32 * a + data[idx]     as u32 * ia) / 255) as u8;
            data[idx + 1] = ((g as u32 * a + data[idx + 1] as u32 * ia) / 255) as u8;
            data[idx + 2] = ((b as u32 * a + data[idx + 2] as u32 * ia) / 255) as u8;
            data[idx + 3] = 255;
        }

        cx += metrics.advance_width as i32;
    }
}
