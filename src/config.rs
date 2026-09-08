use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// How the picture is drawn on screen. Mirrors the values accepted by
/// `org.gnome.desktop.background picture-options`.
pub const PICTURE_OPTIONS: &[&str] = &[
    "none", "wallpaper", "centered", "scaled", "stretched", "zoom", "spanned",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Folders scanned for wallpaper images.
    pub directories: Vec<PathBuf>,
    /// Minutes between automatic rotations (used to build the systemd timer).
    pub interval_minutes: u32,
    /// Never reshow a wallpaper while it sits within the last N shown.
    pub recent_window: usize,
    /// Descend into subdirectories when scanning.
    pub recurse: bool,
    /// Value written to `picture-options`.
    pub picture_options: String,
    /// Random pick when true, otherwise sequential by sorted path.
    pub shuffle: bool,
}

impl Default for Config {
    fn default() -> Self {
        let pictures = dirs::picture_dir()
            .map(|p| p.join("Wallpapers"))
            .into_iter()
            .collect();
        Self {
            directories: pictures,
            interval_minutes: 60,
            recent_window: 10,
            recurse: true,
            picture_options: "zoom".to_string(),
            shuffle: true,
        }
    }
}

impl Config {
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir().context("no XDG config dir")?.join("gnaw");
        Ok(dir.join("config.toml"))
    }

    /// Load config from disk, falling back to defaults if the file is absent.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }

    #[cfg_attr(not(feature = "gui"), allow(dead_code))]
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Clamp interval / window to sane minimums.
    pub fn normalized(mut self) -> Self {
        self.interval_minutes = self.interval_minutes.max(1);
        if !PICTURE_OPTIONS.contains(&self.picture_options.as_str()) {
            self.picture_options = "zoom".to_string();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let dir = std::env::temp_dir().join(format!("gnaw-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let mut cfg = Config::default();
        cfg.directories = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        cfg.interval_minutes = 15;
        cfg.recent_window = 4;
        cfg.shuffle = false;
        cfg.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.directories, cfg.directories);
        assert_eq!(loaded.interval_minutes, 15);
        assert_eq!(loaded.recent_window, 4);
        assert!(!loaded.shuffle);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_yields_defaults() {
        let cfg = Config::load_from(Path::new("/nonexistent/gnaw/config.toml")).unwrap();
        assert_eq!(cfg.interval_minutes, 60);
    }

    #[test]
    fn normalized_clamps() {
        let mut cfg = Config::default();
        cfg.interval_minutes = 0;
        cfg.picture_options = "bogus".into();
        let cfg = cfg.normalized();
        assert_eq!(cfg.interval_minutes, 1);
        assert_eq!(cfg.picture_options, "zoom");
    }
}
