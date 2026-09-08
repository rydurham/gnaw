use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Collect every image file under the given directories, sorted and de-duplicated.
pub fn scan(dirs: &[PathBuf], recurse: bool) -> Vec<PathBuf> {
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();
    for dir in dirs {
        let max_depth = if recurse { usize::MAX } else { 1 };
        for entry in WalkDir::new(dir)
            .max_depth(max_depth)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() && is_image(entry.path()) {
                if let Ok(canon) = entry.path().canonicalize() {
                    out.insert(canon);
                } else {
                    out.insert(entry.path().to_path_buf());
                }
            }
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn finds_images_and_filters_extensions() {
        let base = std::env::temp_dir().join(format!("gnaw-scan-{}", std::process::id()));
        let sub = base.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        touch(&base.join("a.jpg"));
        touch(&base.join("b.PNG"));
        touch(&base.join("notes.txt"));
        touch(&sub.join("c.jpeg"));

        let flat = scan(&[base.clone()], false);
        assert_eq!(flat.len(), 2);

        let deep = scan(&[base.clone()], true);
        assert_eq!(deep.len(), 3);

        std::fs::remove_dir_all(&base).ok();
    }
}
