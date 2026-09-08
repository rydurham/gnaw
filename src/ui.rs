use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use anyhow::Result;
use gtk4 as gtk;
use libadwaita as adw;

use crate::config::{Config, PICTURE_OPTIONS};
use crate::{rotate, systemd};

const APP_ID: &str = "dev.stagerightlabs.Gnaw";

pub fn run() -> Result<()> {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    // Don't let clap args leak into GTK.
    let empty: [String; 0] = [];
    app.run_with_args(&empty);
    Ok(())
}

struct State {
    cfg: Config,
}

fn build_ui(app: &adw::Application) {
    let cfg = Config::load().unwrap_or_default().normalized();
    let state = Rc::new(RefCell::new(State { cfg }));

    let window = adw::PreferencesWindow::builder()
        .application(app)
        .title("Wallpaper Rotation")
        .default_width(520)
        .default_height(640)
        .build();

    let page = adw::PreferencesPage::new();

    // ---- Folders -------------------------------------------------------------
    let folders_group = adw::PreferencesGroup::builder()
        .title("Wallpaper folders")
        .description("Images in these folders are used for rotation")
        .build();

    let folder_rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::new(RefCell::new(Vec::new()));
    let folder_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    folders_group.add(&folder_list);

    // Late-bound self-reference so row callbacks can trigger a rebuild.
    let rebuild_slot: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let rebuild_folders: Rc<dyn Fn()> = {
        let folder_list = folder_list.clone();
        let folder_rows = folder_rows.clone();
        let state = state.clone();
        let rebuild_slot = rebuild_slot.clone();
        Rc::new(move || {
            for row in folder_rows.borrow().iter() {
                folder_list.remove(row);
            }
            folder_rows.borrow_mut().clear();
            let dirs = state.borrow().cfg.directories.clone();
            for (idx, dir) in dirs.into_iter().enumerate() {
                let row = adw::ActionRow::builder()
                    .title(dir.display().to_string())
                    .build();
                let remove = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .tooltip_text("Remove folder")
                    .build();
                let state = state.clone();
                let rebuild_slot = rebuild_slot.clone();
                remove.connect_clicked(move |_| {
                    {
                        let mut s = state.borrow_mut();
                        if idx < s.cfg.directories.len() {
                            s.cfg.directories.remove(idx);
                        }
                        let _ = s.cfg.save();
                    }
                    if let Some(f) = rebuild_slot.borrow().as_ref() {
                        f();
                    }
                });
                row.add_suffix(&remove);
                folder_list.append(&row);
                folder_rows.borrow_mut().push(row);
            }
            if folder_rows.borrow().is_empty() {
                let row = adw::ActionRow::builder()
                    .title("No folders yet")
                    .subtitle("Add one below")
                    .build();
                folder_list.append(&row);
                folder_rows.borrow_mut().push(row);
            }
        })
    };
    *rebuild_slot.borrow_mut() = Some(rebuild_folders.clone());
    rebuild_folders();

    let add_row = adw::ActionRow::builder().title("Add a folder…").build();
    let add_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    add_row.add_suffix(&add_btn);
    folders_group.add(&add_row);

    {
        let state = state.clone();
        let rebuild_folders = rebuild_folders.clone();
        let window = window.clone();
        add_btn.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::builder().title("Choose a wallpaper folder").build();
            let state = state.clone();
            let rebuild_folders = rebuild_folders.clone();
            dialog.select_folder(
                Some(&window),
                gtk::gio::Cancellable::NONE,
                move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            let mut s = state.borrow_mut();
                            if !s.cfg.directories.contains(&path) {
                                s.cfg.directories.push(path);
                                let _ = s.cfg.save();
                            }
                            drop(s);
                            rebuild_folders();
                        }
                    }
                },
            );
        });
    }

    let recurse = adw::SwitchRow::builder()
        .title("Include subfolders")
        .active(state.borrow().cfg.recurse)
        .build();
    {
        let state = state.clone();
        recurse.connect_active_notify(move |sw| {
            let mut s = state.borrow_mut();
            s.cfg.recurse = sw.is_active();
            let _ = s.cfg.save();
        });
    }
    folders_group.add(&recurse);
    page.add(&folders_group);

    // ---- Rotation ----------------------------------------------------------
    let rot_group = adw::PreferencesGroup::builder().title("Rotation").build();

    let interval = adw::SpinRow::builder()
        .title("Interval")
        .subtitle("Minutes between changes")
        .adjustment(&gtk::Adjustment::new(
            state.borrow().cfg.interval_minutes as f64,
            1.0,
            10_080.0,
            5.0,
            30.0,
            0.0,
        ))
        .build();
    {
        let state = state.clone();
        interval.connect_value_notify(move |row| {
            let mut s = state.borrow_mut();
            s.cfg.interval_minutes = row.value().max(1.0) as u32;
            let _ = s.cfg.save();
            let _ = systemd::refresh(&s.cfg);
        });
    }
    rot_group.add(&interval);

    let window_n = adw::SpinRow::builder()
        .title("Avoid repeating")
        .subtitle("Skip the most recently shown N wallpapers")
        .adjustment(&gtk::Adjustment::new(
            state.borrow().cfg.recent_window as f64,
            0.0,
            500.0,
            1.0,
            10.0,
            0.0,
        ))
        .build();
    {
        let state = state.clone();
        window_n.connect_value_notify(move |row| {
            let mut s = state.borrow_mut();
            s.cfg.recent_window = row.value().max(0.0) as usize;
            let _ = s.cfg.save();
        });
    }
    rot_group.add(&window_n);

    let shuffle = adw::SwitchRow::builder()
        .title("Shuffle")
        .subtitle("Random order instead of alphabetical")
        .active(state.borrow().cfg.shuffle)
        .build();
    {
        let state = state.clone();
        shuffle.connect_active_notify(move |sw| {
            let mut s = state.borrow_mut();
            s.cfg.shuffle = sw.is_active();
            let _ = s.cfg.save();
        });
    }
    rot_group.add(&shuffle);

    let opts_model = gtk::StringList::new(PICTURE_OPTIONS);
    let selected = PICTURE_OPTIONS
        .iter()
        .position(|o| *o == state.borrow().cfg.picture_options)
        .unwrap_or(5) as u32;
    let options = adw::ComboRow::builder()
        .title("Fit")
        .model(&opts_model)
        .selected(selected)
        .build();
    {
        let state = state.clone();
        options.connect_selected_notify(move |row| {
            let idx = row.selected() as usize;
            if let Some(v) = PICTURE_OPTIONS.get(idx) {
                let mut s = state.borrow_mut();
                s.cfg.picture_options = v.to_string();
                let _ = s.cfg.save();
            }
        });
    }
    rot_group.add(&options);
    page.add(&rot_group);

    // ---- Status / actions ------------------------------------------------
    let act_group = adw::PreferencesGroup::builder().title("Status").build();

    let status_row = adw::ActionRow::builder().title("Automatic rotation").build();
    let auto_switch = gtk::Switch::builder()
        .valign(gtk::Align::Center)
        .active(systemd::is_active())
        .build();
    status_row.add_suffix(&auto_switch);
    {
        let state = state.clone();
        auto_switch.connect_state_set(move |sw, want| {
            let s = state.borrow();
            let res = if want {
                systemd::install(&s.cfg)
            } else {
                systemd::uninstall()
            };
            if let Err(e) = res {
                eprintln!("timer toggle failed: {e}");
                sw.set_active(!want);
            }
            gtk::glib::Propagation::Stop
        });
    }
    act_group.add(&status_row);

    let next_row = adw::ActionRow::builder()
        .title("Change wallpaper now")
        .activatable(true)
        .build();
    next_row.add_suffix(
        &gtk::Image::from_icon_name("go-next-symbolic"),
    );
    {
        let state = state.clone();
        let toast_parent = window.clone();
        next_row.connect_activated(move |_| {
            let s = state.borrow();
            match rotate::rotate_once(&s.cfg) {
                Ok(r) => {
                    let name = r
                        .chosen
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    toast_parent.add_toast(adw::Toast::new(&format!("Now showing {name}")));
                }
                Err(e) => {
                    toast_parent.add_toast(adw::Toast::new(&format!("{e}")));
                }
            }
        });
    }
    act_group.add(&next_row);
    page.add(&act_group);

    window.add(&page);
    window.present();
}
