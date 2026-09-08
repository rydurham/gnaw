mod config;
mod rotate;
mod scanner;
mod state;
mod systemd;
mod wallpaper;

#[cfg(feature = "gui")]
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

use config::Config;

#[derive(Parser)]
#[command(name = "gnaw", version, about = "Automatic desktop wallpaper rotation for GNOME")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Open the configuration window (default when run with no arguments).
    Ui,
    /// Switch to the next wallpaper now.
    Next,
    /// Show current configuration and rotation status.
    Status,
    /// Install and enable the systemd user timer.
    Install,
    /// Disable and remove the systemd user timer.
    Uninstall,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Ui) {
        Command::Ui => run_ui(),
        Command::Next => cmd_next(),
        Command::Status => cmd_status(),
        Command::Install => cmd_install(),
        Command::Uninstall => cmd_uninstall(),
    }
}

fn cmd_next() -> Result<()> {
    let cfg = Config::load()?.normalized();
    let r = rotate::rotate_once(&cfg)?;
    println!("→ {}  (from {} images)", r.chosen.display(), r.pool_size);
    Ok(())
}

fn cmd_status() -> Result<()> {
    let cfg = Config::load()?.normalized();
    let pool = scanner::scan(&cfg.directories, cfg.recurse);
    let current = wallpaper::current_uri().ok();
    println!("Folders:");
    for d in &cfg.directories {
        println!("  {}", d.display());
    }
    println!("Images found:      {}", pool.len());
    println!("Interval:          {} min", cfg.interval_minutes);
    println!("Avoid last:        {}", cfg.recent_window);
    println!("Order:             {}", if cfg.shuffle { "shuffle" } else { "sequential" });
    println!("Picture options:   {}", cfg.picture_options);
    println!("Timer active:      {}", systemd::is_active());
    if let Some(c) = current {
        println!("Current wallpaper: {c}");
    }
    Ok(())
}

fn cmd_install() -> Result<()> {
    let cfg = Config::load()?.normalized();
    systemd::install(&cfg)?;
    println!("Timer installed and enabled ({} min interval).", cfg.interval_minutes);
    Ok(())
}

fn cmd_uninstall() -> Result<()> {
    systemd::uninstall()?;
    println!("Timer disabled and removed.");
    Ok(())
}

#[cfg(feature = "gui")]
fn run_ui() -> Result<()> {
    ui::run()
}

#[cfg(not(feature = "gui"))]
fn run_ui() -> Result<()> {
    anyhow::bail!("this build was compiled without the `gui` feature; use `gnaw next`/`status`/`install`");
}
