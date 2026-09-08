use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;

const SERVICE: &str = "gnaw-rotate.service";
const TIMER: &str = "gnaw-rotate.timer";

fn unit_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("no XDG config dir")?
        .join("systemd/user");
    Ok(dir)
}

fn exe_path() -> String {
    std::env::current_exe()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "gnaw".to_string())
}

fn systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .context("failed to run `systemctl --user`")?;
    if !status.success() {
        bail!("systemctl --user {:?} failed", args);
    }
    Ok(())
}

/// Write the service + timer unit files using the interval from `cfg`.
pub fn write_units(cfg: &Config) -> Result<()> {
    let dir = unit_dir()?;
    std::fs::create_dir_all(&dir)?;

    let service = format!(
        "[Unit]\n\
         Description=Rotate desktop wallpaper (gnaw)\n\
         After=graphical-session.target\n\n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart={exe} next\n",
        exe = exe_path()
    );
    std::fs::write(dir.join(SERVICE), service)?;

    let timer = format!(
        "[Unit]\n\
         Description=gnaw wallpaper rotation timer\n\n\
         [Timer]\n\
         OnBootSec=2min\n\
         OnUnitActiveSec={interval}min\n\
         Persistent=true\n\n\
         [Install]\n\
         WantedBy=timers.target\n",
        interval = cfg.interval_minutes.max(1)
    );
    std::fs::write(dir.join(TIMER), timer)?;
    Ok(())
}

/// Install and start the timer.
pub fn install(cfg: &Config) -> Result<()> {
    write_units(cfg)?;
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", "--now", TIMER])?;
    Ok(())
}

/// Rewrite units (e.g. after an interval change) and reload if the timer is active.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn refresh(cfg: &Config) -> Result<()> {
    write_units(cfg)?;
    systemctl(&["daemon-reload"])?;
    if is_active() {
        systemctl(&["restart", TIMER])?;
    }
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let _ = systemctl(&["disable", "--now", TIMER]);
    let dir = unit_dir()?;
    let _ = std::fs::remove_file(dir.join(TIMER));
    let _ = std::fs::remove_file(dir.join(SERVICE));
    systemctl(&["daemon-reload"])?;
    Ok(())
}

pub fn is_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", TIMER])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
