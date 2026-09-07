use app::layout_service::LayoutService;
use app::progress::ApplyProgress;
use domain::layout::LayoutDefinition;
use gtk4::{prelude::*, Box, Button, CheckButton, Grid, Label, Orientation, Picture, ProgressBar, ScrolledWindow, Spinner};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;

use crate::resolve_data_dir;

fn set_apply_in_progress(btn: &Button, spinner: &Spinner, progress_bar: &ProgressBar, status_label: &Label) {
    btn.set_sensitive(false);
    btn.set_label("Applying...");
    spinner.set_visible(true);
    spinner.start();
    progress_bar.set_visible(true);
    progress_bar.set_fraction(0.0);
    progress_bar.set_text(Some("Starting"));
    status_label.set_label(
        "Applying layout in background. The window stays responsive; some GNOME changes may appear only after logging out and back in.",
    );
}

fn set_apply_idle(btn: &Button, spinner: &Spinner) {
    spinner.stop();
    spinner.set_visible(false);
    btn.set_sensitive(true);
    btn.set_label("Apply Selected Layout");
}

fn show_error_dialog(window: &gtk4::ApplicationWindow, title: &str, detail: &str) {
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(window)
        .modal(true)
        .message_type(gtk4::MessageType::Error)
        .buttons(gtk4::ButtonsType::Close)
        .text(title)
        .secondary_text(detail)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show();
}

fn show_info_dialog(window: &gtk4::ApplicationWindow, title: &str, detail: &str) {
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(window)
        .modal(true)
        .message_type(gtk4::MessageType::Info)
        .buttons(gtk4::ButtonsType::Ok)
        .text(title)
        .secondary_text(detail)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show();
}

fn wire_logout_action(reload_btn: &Button, window: &gtk4::ApplicationWindow) {
    let reload_window = window.clone();
    reload_btn.connect_clicked(move |_| {
        log::info!("UI Action: 'Log Out To Refresh' clicked");
        let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());

        let dialog = gtk4::MessageDialog::builder()
            .transient_for(&reload_window)
            .modal(true)
            .message_type(gtk4::MessageType::Question)
            .buttons(gtk4::ButtonsType::OkCancel)
            .text("Log Out To Refresh GNOME Session")
            .secondary_text(&format!(
                "Current session type: {}.\n\nOn GNOME Wayland, shell restart is usually blocked, so the reliable way to refresh shell extensions and panel changes is to log out and sign back in.\n\nChoose OK to log out now.",
                session_type
            ))
            .build();

        dialog.connect_response(|dialog, response| {
            if response == gtk4::ResponseType::Ok {
                log::info!("UI Action: confirmed logout for session refresh");
                let _ = infra::executor::execute_bash("gnome-session-quit --logout --no-prompt");
            } else {
                log::info!("UI Action: cancelled logout for session refresh");
            }
            dialog.close();
        });
        dialog.show();
    });
}

fn start_apply_polling(
    child: Child,
    btn: &Button,
    spinner: &Spinner,
    progress_bar: &ProgressBar,
    status_label: &Label,
    window: &gtk4::ApplicationWindow,
    progress_file: PathBuf,
    layout_label: String,
) {
    let child_state = Rc::new(RefCell::new(Some(child)));
    let child_state_poll = child_state.clone();
    let btn_clone = btn.clone();
    let spinner_poll = spinner.clone();
    let status_label_poll = status_label.clone();
    let progress_bar_poll = progress_bar.clone();
    let win_clone_inner = window.clone();

    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        spinner_poll.start();
        if let Some(progress) = read_progress(&progress_file) {
            update_progress_ui(&progress_bar_poll, &status_label_poll, &progress);
        }

        let mut child_slot = child_state_poll.borrow_mut();
        let Some(child) = child_slot.as_mut() else {
            return gtk4::glib::ControlFlow::Break;
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                log::info!("UI Action: apply child exited with status {:?}", status);
                *child_slot = None;
                let _ = fs::remove_file(&progress_file);
                set_apply_idle(&btn_clone, &spinner_poll);

                if status.success() {
                    progress_bar_poll.set_fraction(1.0);
                    progress_bar_poll.set_text(Some("Done"));
                    status_label_poll.set_label(
                        "Layout applied. If the shell still looks unchanged, log out and back in to reload the GNOME session.",
                    );
                    show_info_dialog(
                        &win_clone_inner,
                        "Layout Applied",
                        &format!(
                            "Successfully applied {} layout.\n\nSome GNOME Shell changes are visible only after logging out and back in.",
                            layout_label
                        ),
                    );
                } else {
                    progress_bar_poll.set_text(Some("Failed"));
                    status_label_poll.set_label(
                        "Layout apply failed. Check logs/log.txt for the exact failing step.",
                    );
                    show_error_dialog(
                        &win_clone_inner,
                        "Error Applying Layout",
                        "The background process exited with an error. See logs/log.txt for details.",
                    );
                }

                gtk4::glib::ControlFlow::Break
            }
            Ok(None) => gtk4::glib::ControlFlow::Continue,
            Err(e) => {
                *child_slot = None;
                progress_bar_poll.set_text(Some("Failed"));
                status_label_poll.set_label("Failed while monitoring background apply process.");
                set_apply_idle(&btn_clone, &spinner_poll);
                log::error!("UI Action: failed to poll apply child: {}", e);
                show_error_dialog(&win_clone_inner, "Error Applying Layout", &e.to_string());
                gtk4::glib::ControlFlow::Break
            }
        }
    });
}

fn wire_apply_action(
    apply_btn: &Button,
    selected_layout: Rc<RefCell<Option<LayoutDefinition>>>,
    spinner: &Spinner,
    status_label: &Label,
    progress_bar: &ProgressBar,
    window: &gtk4::ApplicationWindow,
) {
    let selected_clone = selected_layout.clone();
    let win_clone = window.clone();
    let spinner_clone = spinner.clone();
    let status_label_clone = status_label.clone();
    let progress_bar_clone = progress_bar.clone();

    apply_btn.connect_clicked(move |btn| {
        log::info!("UI Action: 'Apply Selected Layout' clicked");
        if let Some(layout) = selected_clone.borrow().as_ref() {
            log::info!("UI Action: applying layout {}", layout.id);
            set_apply_in_progress(btn, &spinner_clone, &progress_bar_clone, &status_label_clone);

            let progress_file = progress_file_path(&layout.id);
            let _ = fs::remove_file(&progress_file);

            match spawn_apply_child(&layout.id, &progress_file) {
                Ok(child) => {
                    start_apply_polling(
                        child,
                        btn,
                        &spinner_clone,
                        &progress_bar_clone,
                        &status_label_clone,
                        &win_clone,
                        progress_file,
                        layout.label.clone(),
                    );
                }
                Err(e) => {
                    set_apply_idle(btn, &spinner_clone);
                    progress_bar_clone.set_visible(false);
                    status_label_clone.set_label("Failed to start background apply process.");
                    log::error!("UI Action: failed to spawn apply child: {}", e);
                    show_error_dialog(&win_clone, "Error Starting Layout Apply", &e.to_string());
                }
            }
        }
    });
}

pub fn build(window: &gtk4::ApplicationWindow) -> gtk4::Widget {
    log::info!("layout_page::build(): begin");
    let container = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .spacing(15)
        .build();

    let title = Label::builder()
        .label("<b>Select GNOME Layout</b>")
        .use_markup(true)
        .halign(gtk4::Align::Center)
        .build();
    container.append(&title);

    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    
    let grid = Grid::builder()
        .row_spacing(20)
        .column_spacing(20)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();
    
    scrolled.set_child(Some(&grid));
    container.append(&scrolled);

    // Initialize layout service
    let _layout_svc = Rc::new(LayoutService::new().expect("Failed to init LayoutService"));
    
    // Resolve data directory:
    // 1. $DESKTOP_EXPERIENCE_DATA_DIR env var (set by run.sh in dist/)
    // 2. Next to binary (dist layout)
    // 3. Dev fallback: relative to project source
    let data_dir = resolve_data_dir();
    log::info!("Using data directory: {:?}", data_dir);
    let layouts = LayoutService::load_layouts(&data_dir).unwrap_or_default();
    log::info!("layout_page::build(): loaded {} layouts", layouts.len());
    
    let selected_layout = Rc::new(RefCell::new(None::<LayoutDefinition>));
    let mut first_radio: Option<CheckButton> = None;

    for (i, layout) in layouts.iter().enumerate() {
        let v_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .halign(gtk4::Align::Center)
            .build();

        // 1. Image preview
        let preview_name = layout.preview.as_deref().unwrap_or_else(|| &layout.id);
        let preview_filename = if Path::new(preview_name).extension().is_some() {
            preview_name.to_string()
        } else {
            format!("{}preview.svg", preview_name)
        };
        let preview_path = data_dir.join("pictures").join(preview_filename);
        let picture = Picture::for_filename(&preview_path);
        picture.set_size_request(200, 150);
        v_box.append(&picture);

        // 2. Radio button
        let radio = if let Some(first) = &first_radio {
            CheckButton::builder()
                .group(first)
                .label(&layout.label)
                .build()
        } else {
            let r = CheckButton::builder()
                .label(&layout.label)
                .build();
            // Default selection
            *selected_layout.borrow_mut() = Some(layout.clone());
            r.set_active(true);
            first_radio = Some(r.clone());
            r
        };

        let layout_clone = layout.clone();
        let selected_clone = selected_layout.clone();
        radio.connect_toggled(move |btn| {
            if btn.is_active() {
                log::info!("layout_page::build(): selected layout id={}", layout_clone.id);
                *selected_clone.borrow_mut() = Some(layout_clone.clone());
            }
        });

        v_box.append(&radio);

        // Calculate grid (2 columns)
        let row = (i / 2) as i32;
        let col = (i % 2) as i32;
        grid.attach(&v_box, col, row, 1, 1);
    }

    let apply_btn = Button::builder()
        .label("Apply Selected Layout")
        .halign(gtk4::Align::Center)
        .margin_top(15)
        .build();
    apply_btn.add_css_class("suggested-action");

    let spinner = Spinner::builder()
        .spinning(false)
        .visible(false)
        .build();

    let status_label = Label::builder()
        .label("")
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .halign(gtk4::Align::Center)
        .css_classes(vec!["dim-label"])
        .build();

    let progress_bar = ProgressBar::builder()
        .hexpand(true)
        .show_text(true)
        .fraction(0.0)
        .visible(false)
        .build();

    let reload_btn = Button::builder()
        .label("Log Out To Refresh")
        .halign(gtk4::Align::Center)
        .margin_top(15)
        .build();

    wire_apply_action(&apply_btn, selected_layout.clone(), &spinner, &status_label, &progress_bar, window);
    wire_logout_action(&reload_btn, window);

    let btn_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .valign(gtk4::Align::Center)
        .halign(gtk4::Align::Center)
        .spacing(15)
        .build();
    btn_box.append(&apply_btn);
    btn_box.append(&spinner);
    btn_box.append(&reload_btn);

    container.append(&btn_box);
    container.append(&progress_bar);
    container.append(&status_label);

    log::info!("layout_page::build(): done");
    container.upcast()
}

fn spawn_apply_child(layout_id: &str, progress_file: &PathBuf) -> anyhow::Result<Child> {
    let current_exe = std::env::current_exe()?;
    log::info!(
        "spawn_apply_child(): exe={:?} layout_id={} data_dir={:?} progress_file={:?}",
        current_exe,
        layout_id,
        std::env::var("DESKTOP_EXPERIENCE_DATA_DIR").ok(),
        progress_file
    );

    let child = Command::new(current_exe)
        .arg("--apply-layout")
        .arg(layout_id)
        .env("DESKTOP_EXPERIENCE_PROGRESS_FILE", progress_file)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    Ok(child)
}

fn progress_file_path(layout_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("desktop-experience-switcher-{}.progress.json", layout_id))
}

fn read_progress(path: &PathBuf) -> Option<ApplyProgress> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn update_progress_ui(progress_bar: &ProgressBar, status_label: &Label, progress: &ApplyProgress) {
    let total = progress.total_steps.max(1);
    let fraction = (progress.current_step as f64 / total as f64).clamp(0.0, 1.0);
    progress_bar.set_fraction(fraction);
    progress_bar.set_text(Some(&format!("{}/{}", progress.current_step, progress.total_steps)));

    let mut lines = Vec::new();
    if !progress.stage_label.trim().is_empty() {
        lines.push(progress.stage_label.trim().to_string());
    }
    if let Some(target) = &progress.current_target {
        if !target.trim().is_empty() {
            lines.push(format!("Current target: {}", target.trim()));
        }
    }
    if !progress.detail.trim().is_empty() {
        lines.push(progress.detail.trim().to_string());
    }

    status_label.set_label(&lines.join("\n"));
}
