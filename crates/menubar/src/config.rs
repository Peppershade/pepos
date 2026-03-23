use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub menubar: MenubarConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MenubarConfig {
    pub height: u32,
    pub font: String,
    pub font_size: f32,
    pub background: String,
    pub foreground: String,
    pub accent: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            menubar: MenubarConfig {
                height: 28,
                font: "Inter".to_string(),
                font_size: 13.0,
                background: "#1a1a1a".to_string(),
                foreground: "#e8e8e8".to_string(),
                accent: "#5e9bff".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let path = format!("{}/.config/pepos/pepos.toml", home);

        if let Ok(contents) = std::fs::read_to_string(&path) {
            match toml::from_str(&contents) {
                Ok(cfg) => {
                    tracing::info!("loaded config from {}", path);
                    return cfg;
                }
                Err(e) => tracing::warn!("bad config at {}: {}", path, e),
            }
        }

        tracing::info!("using default config");
        Config::default()
    }
}

/// Parse a CSS-style hex color string like "#1a1a1a" into (r, g, b, a).
pub fn hex_to_rgba(hex: &str) -> (u8, u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let parse = |s: &str| u8::from_str_radix(s, 16).unwrap_or(0);
    let r = parse(&h[0..2]);
    let g = parse(&h[2..4]);
    let b = parse(&h[4..6]);
    let a = if h.len() >= 8 { parse(&h[6..8]) } else { 255 };
    (r, g, b, a)
}
