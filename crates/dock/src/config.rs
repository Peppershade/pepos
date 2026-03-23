use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub dock: DockConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DockConfig {
    pub icon_size: u32,
    pub padding: u32,
    pub background: String,
    pub foreground: String,
    pub corner_radius: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dock: DockConfig {
                icon_size: 52,
                padding: 8,
                background: "#1a1a1a".to_string(),
                foreground: "#e8e8e8".to_string(),
                corner_radius: 12.0,
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let path = format!("{}/.config/pepos/pepos.toml", home);
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str(&contents) {
                return cfg;
            }
        }
        Config::default()
    }

    pub fn bar_height(&self) -> u32 {
        self.dock.padding * 2 + self.dock.icon_size
    }
}

pub fn hex_to_rgba(hex: &str) -> (u8, u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let p = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
    let a = if h.len() >= 8 { p(&h[6..8]) } else { 255 };
    (p(&h[0..2]), p(&h[2..4]), p(&h[4..6]), a)
}
