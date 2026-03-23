// Discovers installed applications by reading .desktop files.
// .desktop files live in /usr/share/applications/ and follow the freedesktop spec.

#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub exec: String,
}

pub fn load() -> Vec<App> {
    let dirs = ["/usr/share/applications", "/usr/local/share/applications"];
    let mut apps = Vec::new();

    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                if let Some(app) = parse(&path) {
                    apps.push(app);
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name == b.name);
    apps
}

fn parse(path: &std::path::Path) -> Option<App> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut is_app = false;
    let mut hidden = false;

    // .desktop files are INI-like; we only care about the [Desktop Entry] section
    let mut in_entry = false;
    for line in content.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" { in_entry = true; continue; }
        if line.starts_with('[') { in_entry = false; continue; }
        if !in_entry { continue; }

        if line.starts_with("Type=")       { is_app = line == "Type=Application"; }
        if line.starts_with("NoDisplay=")  { hidden = line == "NoDisplay=true"; }
        if line.starts_with("Hidden=")     { hidden |= line == "Hidden=true"; }
        if line.starts_with("Name=") && name.is_none() {
            name = Some(line[5..].to_string());
        }
        if line.starts_with("Exec=") && exec.is_none() {
            exec = Some(strip_field_codes(&line[5..]));
        }
    }

    if is_app && !hidden {
        Some(App { name: name?, exec: exec? })
    } else {
        None
    }
}

/// Remove freedesktop field codes like %f, %u, %F, %U from exec strings.
fn strip_field_codes(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
}
