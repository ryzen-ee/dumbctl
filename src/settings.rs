use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

impl Theme {
    pub fn next(self) -> Self {
        match self {
            Theme::System => Theme::Dark,
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::System,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }

    pub fn detect_system_theme() -> Self {
        if let Ok(color_fg_bg) = std::env::var("COLORFGBG") {
            if color_fg_bg.starts_with('0') || color_fg_bg.starts_with("15") {
                return Theme::Light;
            }
        }
        Theme::Dark
    }

    pub fn colors(&self) -> ThemeColors {
        match self {
            Theme::System => Theme::detect_system_theme().colors(),
            Theme::Dark => ThemeColors {
                bg: ratatui::style::Color::Black,
                fg: ratatui::style::Color::White,
                border: ratatui::style::Color::White,
                block: ratatui::style::Color::White,
                selected: ratatui::style::Color::LightBlue,
                title: ratatui::style::Color::Cyan,
                healthy: ratatui::style::Color::Green,
                warning: ratatui::style::Color::Yellow,
                critical: ratatui::style::Color::Red,
                muted: ratatui::style::Color::DarkGray,
            },
            Theme::Light => ThemeColors {
                bg: ratatui::style::Color::White,
                fg: ratatui::style::Color::Black,
                border: ratatui::style::Color::DarkGray,
                block: ratatui::style::Color::Black,
                selected: ratatui::style::Color::Blue,
                title: ratatui::style::Color::Blue,
                healthy: ratatui::style::Color::Green,
                warning: ratatui::style::Color::Magenta,
                critical: ratatui::style::Color::Red,
                muted: ratatui::style::Color::DarkGray,
            },
        }
    }
}

#[allow(dead_code)]
pub struct ThemeColors {
    pub bg: ratatui::style::Color,
    pub fg: ratatui::style::Color,
    pub border: ratatui::style::Color,
    pub block: ratatui::style::Color,
    pub selected: ratatui::style::Color,
    pub title: ratatui::style::Color,
    pub healthy: ratatui::style::Color,
    pub warning: ratatui::style::Color,
    pub critical: ratatui::style::Color,
    pub muted: ratatui::style::Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub export_path: String,
    pub theme: Theme,
    pub auto_refresh_interval: u32,
    pub benchmark_size_mb: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            export_path: String::new(),
            theme: Theme::System,
            auto_refresh_interval: 0,
            benchmark_size_mb: 512,
        }
    }
}

impl Settings {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("dumbctl").join("settings.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(contents) = fs::read_to_string(&path) {
                    if let Ok(settings) = serde_json::from_str(&contents) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config path")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let contents = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, contents).map_err(|e| e.to_string())?;

        Ok(())
    }
}
