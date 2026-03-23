use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub launcher: LauncherConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LauncherConfig {
    pub width:        u32,
    pub max_results:  usize,
    pub background:   String,
    pub foreground:   String,
    pub accent:       String,
    pub corner_radius: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            launcher: LauncherConfig {
                width:        640,
                max_results:  8,
                background:   "#1a1a1a".to_string(),
                foreground:   "#e8e8e8".to_string(),
                accent:       "#5e9bff".to_string(),
                corner_radius: 12.0,
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let path = format!("{}/.config/pepos/pepos.toml", home);
        if let Ok(s) = std::fs::read_to_string(path) {
            if let Ok(c) = toml::from_str(&s) { return c; }
        }
        Config::default()
    }

    /// Total height = input row + (max_results * row_height)
    pub fn panel_height(&self) -> u32 {
        let row_h = 36_u32;
        row_h + self.launcher.max_results as u32 * row_h
    }
}

pub fn hex_to_rgba(hex: &str) -> (u8, u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let p = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
    let a = if h.len() >= 8 { p(&h[6..8]) } else { 255 };
    (p(&h[0..2]), p(&h[2..4]), p(&h[4..6]), a)
}
