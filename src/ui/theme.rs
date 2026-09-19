use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub name: String,
    pub accent: String,
    pub foreground: String,
    pub background: String,
    pub dark_background: String,
    pub muted: String,
    pub cyan: String,
    pub magenta: String,
    pub blue: String,
    pub active_border: String,
    pub selection: String,
    pub bright_red: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            name: "Omarchy".to_string(),
            accent: "#00f0ff".to_string(),
            foreground: "#cacccc".to_string(),
            background: "#101315".to_string(),
            dark_background: "#080a0b".to_string(),
            muted: "#707880".to_string(),
            cyan: "#00f0ff".to_string(),
            magenta: "#d946ef".to_string(),
            blue: "#38bdf8".to_string(),
            active_border: "#a8adb0".to_string(),
            selection: "#343d41".to_string(),
            bright_red: "#de6145".to_string(),
        }
    }
}

pub fn load_omarchy_theme() -> ThemeColors {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let state_dir = PathBuf::from(&home).join(".local/state/omarchy/current");
    let name_path = state_dir.join("theme.name");
    let colors_path = state_dir.join("theme/colors.toml");

    let mut theme = ThemeColors::default();

    if name_path.is_file() {
        if let Ok(content) = fs::read_to_string(&name_path) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                theme.name = trimmed.to_string();
            }
        }
    }

    if colors_path.is_file() {
        if let Ok(content) = fs::read_to_string(&colors_path) {
            let mut map = HashMap::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    let key = k.trim();
                    let raw_val = v.trim();
                    let clean_val = if (raw_val.starts_with('"') && raw_val.len() >= 2)
                        || (raw_val.starts_with('\'') && raw_val.len() >= 2)
                    {
                        let quote = raw_val.chars().next().unwrap();
                        if let Some(end_idx) = raw_val[1..].find(quote) {
                            &raw_val[1..=end_idx]
                        } else {
                            raw_val.trim_matches(|c| c == '"' || c == '\'')
                        }
                    } else {
                        raw_val.split_whitespace().next().unwrap_or(raw_val)
                    };
                    map.insert(key.to_string(), clean_val.to_string());
                }
            }

            if let Some(v) = map.get("accent") {
                theme.accent = v.clone();
            }
            if let Some(v) = map.get("foreground") {
                theme.foreground = v.clone();
            }
            if let Some(v) = map.get("background") {
                theme.background = v.clone();
            }
            if let Some(v) = map
                .get("dark_background")
                .or_else(|| map.get("darker_background"))
            {
                theme.dark_background = v.clone();
            }
            if let Some(v) = map.get("muted") {
                theme.muted = v.clone();
            }
            if let Some(v) = map.get("bright_cyan").or_else(|| map.get("cyan")) {
                theme.cyan = v.clone();
            }
            if let Some(v) = map.get("bright_magenta").or_else(|| map.get("magenta")) {
                theme.magenta = v.clone();
            }
            if let Some(v) = map.get("bright_blue").or_else(|| map.get("blue")) {
                theme.blue = v.clone();
            }
            if let Some(v) = map.get("active_border_color") {
                theme.active_border = v.clone();
            }
            if let Some(v) = map.get("selection") {
                theme.selection = v.clone();
            }
            if let Some(v) = map.get("bright_red").or_else(|| map.get("red")) {
                theme.bright_red = v.clone();
            }
        }
    }

    debug!(
        "Loaded Omarchy theme: '{}' (accent: {})",
        theme.name, theme.accent
    );
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let theme = ThemeColors::default();
        assert_eq!(theme.name, "Omarchy");
        assert_eq!(theme.accent, "#00f0ff");
    }

    #[test]
    fn test_load_theme() {
        let theme = load_omarchy_theme();
        assert!(!theme.name.is_empty());
        assert!(!theme.accent.is_empty());
    }
}
