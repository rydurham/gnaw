# Gnome Automatic Wallpaper

Automatic desktop wallpaper rotation for GNOME, written in Rust.

- Rotates the wallpaper on a configurable interval via a **systemd user timer**.
- Recently-shown wallpapers (and the current one) are excluded from the next pick —
  configurable sliding window of the last *N*.
- Sets both the light and dark wallpaper keys (`picture-uri`, `picture-uri-dark`) to the
  same image.
- libadwaita configuration window (`gnaw ui`), plus a full CLI.
- Only runtime dependency is `gsettings` + the `org.gnome.desktop.background` schema,
  present on every GNOME desktop — no root, no GNOME extension.

## Build

System packages (Debian/Ubuntu):

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev build-essential pkg-config
```

Fedora: `sudo dnf install gtk4-devel libadwaita-devel`.
Arch: `sudo pacman -S gtk4 libadwaita`.

Then:

```bash
cargo install --path .
```

This installs `gnaw` to `~/.cargo/bin`. Building without the GUI (CLI only, no GTK
dependencies) is possible with `cargo build --release --no-default-features`.

## Desktop entry

To make the config window show up in your app launcher (GNOME Activities, "Wallpaper
Rotation"), install the bundled desktop file after `gnaw` is on your `PATH`:

```bash
install -Dm644 data/dev.stagerightlabs.Gnaw.desktop ~/.local/share/applications/dev.stagerightlabs.Gnaw.desktop
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

For a system-wide install, drop it in `/usr/share/applications/` instead (needs root).
The entry runs `gnaw ui`, so `gnaw` must be resolvable from your session's `PATH` —
`~/.cargo/bin` usually is. Remove it with:

```bash
rm ~/.local/share/applications/dev.stagerightlabs.Gnaw.desktop
```

## Usage

```bash
gnaw ui         # open the configuration window (also the default with no args)
gnaw next       # switch to the next wallpaper now
gnaw status     # show configuration and whether the timer is active
gnaw install    # write + enable the systemd user timer
gnaw uninstall  # disable + remove the timer
```

Quick start:

```bash
gnaw ui         # add your wallpaper folder(s), set the interval
gnaw install    # start automatic rotation
```

## Configuration

`~/.config/gnaw/config.toml` (created/edited by the UI, but plain TOML):

```toml
directories = ["/home/you/Pictures/Wallpapers"]
interval_minutes = 60
recent_window = 10       # never reshow while within the last N shown
recurse = true           # descend into subfolders
picture_options = "zoom" # none | wallpaper | centered | scaled | stretched | zoom | spanned
shuffle = true           # random pick; false = alphabetical by path
```

### Fit options (`picture_options`)

This is passed straight through to GNOME's `org.gnome.desktop.background picture-options`
key, which controls how each image is mapped onto the screen:

| Value       | Meaning                                                                                                                                                          |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `none`      | Don't draw the image at all — just the solid background color.                                                                                                   |
| `wallpaper` | Tile the image at its native size, repeating it to fill the screen.                                                                                              |
| `centered`  | Place the image at its native size in the middle of the screen; no scaling. Smaller images leave a border, larger ones are cropped.                              |
| `scaled`    | Scale the image up or down to fit the screen while keeping its aspect ratio. The whole image is visible; mismatched aspect ratios leave bars (letter/pillarbox). |
| `stretched` | Scale the image to fill the screen exactly, ignoring aspect ratio. Fills everything but distorts images whose proportions don't match the display.               |
| `zoom`      | Scale the image to cover the whole screen while keeping its aspect ratio, cropping whatever overflows. No bars, no distortion. gnaw's default.                   |
| `spanned`   | Like `zoom`, but the image is stretched across all monitors as one surface rather than repeated per-monitor.                                                     |

For most photo wallpapers `zoom` is the best choice. Use `scaled` if you'd rather see
the entire image and can live with bars, or `centered` for pixel art / images you don't
want resampled.

Rotation history lives at `~/.local/state/gnaw/history.json`.

Changing the interval (in the UI or the file) requires re-running `gnaw install` — or the
UI does it for you — so systemd picks up the new schedule.

## How rotation picks an image

1. Scan `directories` for `jpg/jpeg/png/webp/bmp`.
2. Remove the current wallpaper and the last `recent_window` entries from the pool.
3. If that leaves nothing (small library), fall back to "anything but the current one".
4. Pick randomly (`shuffle`) or take the next path in sorted order.
5. Apply, then append to history.
