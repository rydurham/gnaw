# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`gnaw` is a single Rust binary that rotates the GNOME desktop wallpaper on a schedule. There is no daemon: a **systemd user timer** (`gnaw-rotate.timer`) fires `gnaw next` every `interval_minutes`. The binary also ships a libadwaita config GUI (`gnaw ui`, the default subcommand).

## Commands

```bash
cargo build --release                       # full build (needs GTK4 + libadwaita dev packages)
cargo build --release --no-default-features # CLI only, no GTK/adwaita dependency
cargo test                                  # all unit tests (pure logic, no GTK needed)
cargo test choose                           # single test by name substring
cargo test --no-default-features            # tests for the non-GUI build
cargo install --path .                      # install `gnaw` to ~/.cargo/bin
```

System packages for the GUI build: `libgtk-4-dev libadwaita-1-dev` (Debian/Ubuntu). The `gtk4` crate needs its `v4_10` feature for `FileDialog` — already set in `Cargo.toml`.

Runtime deps for rotation itself: only `gsettings` and the `org.gnome.desktop.background` schema.

## Architecture

The GUI is gated behind the `gui` feature (on by default). `src/main.rs` compiles a stub `run_ui()` when it's off, so **every non-`ui.rs` module must build without GTK**. Keep GTK/adwaita imports confined to `ui.rs`.

Data flow for a rotation (`rotate::rotate_once`):
1. `scanner::scan` walks `cfg.directories` (canonicalizing, de-duping via `BTreeSet`) for jpg/jpeg/png/webp/bmp.
2. `state::History` (`~/.local/state/gnaw/history.json`) supplies the last `recent_window` shown paths.
3. `wallpaper::current_uri` reads the live `picture-uri` via `gsettings`; `rotate::uri_to_path` decodes it.
4. `rotate::choose` — the pure, unit-tested core — excludes current + recent, then **falls back progressively** (anything-but-current, then the whole pool) so a tiny library still rotates. Shuffle = random pick; otherwise next path after current in sorted order, wrapping.
5. `wallpaper::set` writes `picture-uri`, `picture-uri-dark` (both to the same image) and `picture-options` via `gsettings`.
6. `History::record` appends (dedup, capped at `max(recent_window*2, 50)`).

`config::Config` (`~/.config/gnaw/config.toml`, serde `#[serde(default)]` so partial files load). Always call `.normalized()` after `load()` — it clamps `interval_minutes >= 1` and resets an invalid `picture_options` to `zoom`. `PICTURE_OPTIONS` is the source-of-truth list, passed straight to GNOME.

`systemd.rs` writes unit files into `~/.config/systemd/user/` using the **absolute path of the running exe** (`current_exe()`), then shells out to `systemctl --user`. Changing the interval requires rewriting units + `daemon-reload` + `restart` — `systemd::refresh` does this and the UI calls it on every interval change; the CLI requires a manual `gnaw install`.

### `ui.rs` specifics

`AdwPreferencesWindow` built imperatively. Every row's change callback writes config immediately (`s.cfg.save()`) — there is no explicit Save button. The folder list uses a late-bound `rebuild_slot: Rc<RefCell<Option<Rc<dyn Fn()>>>>` so per-row remove buttons can trigger a full list rebuild. `run()` passes an empty argv to `app.run_with_args` so clap's parsed args don't leak into GTK.

## Conventions

- Errors: `anyhow` everywhere, `.context()`/`.with_context()` on all I/O and subprocess calls.
- Path/URI handling is done by hand (`path_to_uri`, `uri_to_path`) — percent-encoding only the reserved bytes. Both directions have tests; keep them in sync.
- Tests live in `#[cfg(test)]` modules in the same file and use `std::env::temp_dir()` with a pid-suffixed dir.
- App ID / desktop file basename: `dev.stagerightlabs.Gnaw`.
