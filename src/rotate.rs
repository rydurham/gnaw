use anyhow::{bail, Result};
use rand::seq::SliceRandom;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::state::History;
use crate::{scanner, wallpaper};

/// Decode a `file://` URI into a filesystem path (percent-decoding included).
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(rest.len());
    let mut chars = rest.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h = chars.next()?;
            let l = chars.next()?;
            let hex = |c: u8| (c as char).to_digit(16);
            bytes.push((hex(h)? * 16 + hex(l)?) as u8);
        } else {
            bytes.push(b);
        }
    }
    Some(PathBuf::from(String::from_utf8(bytes).ok()?))
}

/// Choose the next wallpaper given the full pool, exclusions and the current one.
///
/// Split out from I/O so it can be unit-tested. Returns `None` only when the pool
/// is empty.
pub fn choose<'a>(
    pool: &'a [PathBuf],
    recent: &[&PathBuf],
    current: Option<&Path>,
    shuffle: bool,
) -> Option<&'a PathBuf> {
    if pool.is_empty() {
        return None;
    }

    let excluded = |p: &PathBuf| {
        Some(p.as_path()) == current || recent.iter().any(|r| r.as_path() == p.as_path())
    };

    let candidates: Vec<&PathBuf> = pool.iter().filter(|p| !excluded(p)).collect();

    // Fall back progressively so a tiny library still rotates.
    let candidates = if !candidates.is_empty() {
        candidates
    } else {
        let no_current: Vec<&PathBuf> =
            pool.iter().filter(|p| Some(p.as_path()) != current).collect();
        if !no_current.is_empty() {
            no_current
        } else {
            pool.iter().collect()
        }
    };

    if shuffle {
        candidates.choose(&mut rand::thread_rng()).copied()
    } else {
        // Sequential: first candidate after `current` in sorted order, wrapping.
        let after = current.and_then(|c| {
            candidates
                .iter()
                .find(|p| p.as_path() > c)
                .copied()
        });
        after.or_else(|| candidates.first().copied())
    }
}

pub struct Rotation {
    pub chosen: PathBuf,
    pub pool_size: usize,
}

/// Perform one rotation: scan, pick, apply, persist history.
pub fn rotate_once(cfg: &Config) -> Result<Rotation> {
    let pool = scanner::scan(&cfg.directories, cfg.recurse);
    if pool.is_empty() {
        bail!(
            "no images found in configured folders: {}",
            cfg.directories
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let mut history = History::load()?;
    let recent = history.recent(cfg.recent_window);
    let current = wallpaper::current_uri().ok().and_then(|u| uri_to_path(&u));

    let chosen = choose(&pool, &recent, current.as_deref(), cfg.shuffle)
        .expect("pool is non-empty")
        .clone();
    drop(recent);

    wallpaper::set(&chosen, &cfg.picture_options)?;

    let cap = (cfg.recent_window * 2).max(50);
    history.record(chosen.clone(), cap);
    history.save()?;

    Ok(Rotation {
        chosen,
        pool_size: pool.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pb(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn decodes_uri() {
        assert_eq!(
            uri_to_path("file:///home/x/a%20b.jpg"),
            Some(pb("/home/x/a b.jpg"))
        );
    }

    #[test]
    fn excludes_recent_and_current() {
        let pool = vec![pb("/w/1.jpg"), pb("/w/2.jpg"), pb("/w/3.jpg"), pb("/w/4.jpg")];
        let recent = vec![&pool[0], &pool[1]];
        let current = pool[2].clone();
        for _ in 0..20 {
            let pick = choose(&pool, &recent, Some(&current), true).unwrap();
            assert_eq!(pick, &pool[3]);
        }
    }

    #[test]
    fn fallback_when_all_excluded() {
        let pool = vec![pb("/w/1.jpg"), pb("/w/2.jpg")];
        let recent = vec![&pool[0], &pool[1]];
        let current = pool[0].clone();
        // everything is recent; fallback = anything but current
        let pick = choose(&pool, &recent, Some(&current), true).unwrap();
        assert_eq!(pick, &pool[1]);
    }

    #[test]
    fn single_image_still_returns() {
        let pool = vec![pb("/w/1.jpg")];
        let current = pool[0].clone();
        let pick = choose(&pool, &[], Some(&current), true).unwrap();
        assert_eq!(pick, &pool[0]);
    }

    #[test]
    fn sequential_advances_in_order() {
        let pool = vec![pb("/w/1.jpg"), pb("/w/2.jpg"), pb("/w/3.jpg")];
        let pick = choose(&pool, &[], Some(&pool[0]), false).unwrap();
        assert_eq!(pick, &pool[1]);
        let wrap = choose(&pool, &[], Some(&pool[2]), false).unwrap();
        assert_eq!(wrap, &pool[0]);
    }
}
