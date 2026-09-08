use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Rotation history, most-recently-shown last.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub shown: VecDeque<PathBuf>,
}

impl History {
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::state_dir()
            .or_else(dirs::data_dir)
            .context("no XDG state dir")?
            .join("gnaw");
        Ok(dir.join("history.json"))
    }

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
        Ok(serde_json::from_str(&text).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// The last `n` shown paths — these are excluded from consideration.
    pub fn recent(&self, n: usize) -> Vec<&PathBuf> {
        let start = self.shown.len().saturating_sub(n);
        self.shown.iter().skip(start).collect()
    }

    /// Record a newly shown wallpaper and cap stored history length.
    pub fn record(&mut self, path: PathBuf, cap: usize) {
        self.shown.retain(|p| p != &path);
        self.shown.push_back(path);
        while self.shown.len() > cap {
            self.shown.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_dedupes_and_caps() {
        let mut h = History::default();
        for i in 0..10 {
            h.record(PathBuf::from(format!("/w/{i}.jpg")), 5);
        }
        assert_eq!(h.shown.len(), 5);
        h.record(PathBuf::from("/w/7.jpg"), 5);
        assert_eq!(h.shown.len(), 5);
        assert_eq!(h.shown.back().unwrap(), &PathBuf::from("/w/7.jpg"));
        // 7 should appear only once
        assert_eq!(h.shown.iter().filter(|p| p.ends_with("7.jpg")).count(), 1);
    }

    #[test]
    fn recent_returns_tail() {
        let mut h = History::default();
        for i in 0..5 {
            h.record(PathBuf::from(format!("/w/{i}.jpg")), 100);
        }
        let r = h.recent(2);
        assert_eq!(r.len(), 2);
        assert!(r[1].ends_with("4.jpg"));
    }
}
