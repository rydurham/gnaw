use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

const SCHEMA: &str = "org.gnome.desktop.background";

fn gsettings(args: &[&str]) -> Result<String> {
    let out = Command::new("gsettings")
        .args(args)
        .output()
        .context("failed to run `gsettings` (is GNOME / glib installed?)")?;
    if !out.status.success() {
        bail!(
            "gsettings {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn path_to_uri(path: &Path) -> String {
    // gsettings expects a file:// URI. Percent-encoding is not required by GNOME
    // for the common case, but spaces must be handled; encode the few reserved bytes.
    let mut s = String::from("file://");
    for byte in path.to_string_lossy().as_bytes() {
        match byte {
            b'/' | b'-' | b'_' | b'.' | b'~' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => {
                s.push(*byte as char)
            }
            _ => s.push_str(&format!("%{byte:02X}")),
        }
    }
    s
}

/// The URI currently set as the light-mode wallpaper.
pub fn current_uri() -> Result<String> {
    let raw = gsettings(&["get", SCHEMA, "picture-uri"])?;
    Ok(raw.trim_matches('\'').to_string())
}

/// Point both the light and dark wallpaper keys at `path` and apply `picture_options`.
pub fn set(path: &Path, picture_options: &str) -> Result<()> {
    let uri = path_to_uri(path);
    gsettings(&["set", SCHEMA, "picture-uri", &uri])?;
    gsettings(&["set", SCHEMA, "picture-uri-dark", &uri])?;
    gsettings(&["set", SCHEMA, "picture-options", picture_options])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_spaces() {
        let uri = path_to_uri(Path::new("/home/x/My Wallpapers/a b.jpg"));
        assert_eq!(uri, "file:///home/x/My%20Wallpapers/a%20b.jpg");
    }
}
