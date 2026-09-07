use app::{history_service::HistoryService, snapshot_service::SnapshotService};
use chrono::Local;
use domain::history::ChangeKind;
use gtk4::{prelude::*, Box, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use std::rc::Rc;

/// Buduje wiersz historii i dodaje go do listboxa
fn populate_history(
    listbox: &ListBox,
    snapshot_svc: Rc<SnapshotService>,
    window: &gtk4::ApplicationWindow,
) {
    // Wyczyść obecną zawartość
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }

    let history_svc = match HistoryService::new() {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to init HistoryService: {}", e);
            let err_lbl = Label::builder().label("Błąd ładowania historii.").build();
            listbox.append(&err_lbl);
            return;
        }
    };

    let history = history_svc.get_history().unwrap_or_default();

    if history.is_empty() {
        let empty = Label::builder()
            .label("Brak historii. Zmień layout lub stwórz snapshot.")
            .halign(gtk4::Align::Center)
            .margin_top(20)
            .build();
        empty.add_css_class("dim-label");
        listbox.append(&empty);
        return;
    }

    // Najnowsze na górze
    let mut sorted = history;
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    for entry in sorted {
        let row = ListBoxRow::builder().build();

        let row_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(15)
            .margin_start(10)
            .margin_end(10)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        // Czas
        let local_time = entry.timestamp.with_timezone(&Local);
        let time_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();

        // Opis zdarzenia
        let (icon, kind_str) = match &entry.kind {
            ChangeKind::AppLaunch => ("🚀", "Uruchomienie aplikacji (Auto-Snapshot)".to_string()),
            ChangeKind::LayoutApplied(id) => ("🎨", format!("Zastosowano Layout: {}", id)),
            ChangeKind::LayoutApplyBlocked(id) => ("⛔", format!("Zablokowano wdrożenie layoutu: {}", id)),
            ChangeKind::LayoutApplyFailed(id) => ("❌", format!("Błąd wdrażania layoutu: {}", id)),
            ChangeKind::LayoutRolledBack(id) => ("↩", format!("Wycofano zmiany layoutu: {}", id)),
            ChangeKind::SnapshotRestored(id) => ("⏪", format!("Przywrócono Snapshot: {}…", &id.to_string()[..8])),
            ChangeKind::ManualSnapshot(id) => ("📸", format!("Ręczny Snapshot: {}…", &id.to_string()[..8])),
        };

        let text_box = Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .build();

        let title_lbl = Label::builder()
            .label(&format!("{} {}", icon, kind_str))
            .halign(gtk4::Align::Start)
            .build();
        title_lbl.add_css_class("heading");

        let time_lbl = Label::builder()
            .label(&time_str)
            .halign(gtk4::Align::Start)
            .build();
        time_lbl.add_css_class("dim-label");

        text_box.append(&title_lbl);
        text_box.append(&time_lbl);
        row_box.append(&text_box);

        // Przycisk Restore jeśli jest snapshot do przywrócenia
        if let Some(snapshot_id) = entry.snapshot_id_before.or(entry.snapshot_id_after) {
            let actions_box = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .valign(gtk4::Align::Center)
                .build();

            let restore_btn = Button::builder()
                .label("Przywróć")
                .valign(gtk4::Align::Center)
                .build();

            let snap_svc_clone = snapshot_svc.clone();
            let win_clone = window.clone();

            restore_btn.connect_clicked(move |_| {
                log::info!("UI Action: 'Restore Config' clicked for snapshot {}", snapshot_id);
                match snap_svc_clone.restore_snapshot(&snapshot_id) {
                    Ok(_) => {
                        let dialog = gtk4::MessageDialog::builder()
                            .transient_for(&win_clone)
                            .modal(true)
                            .message_type(gtk4::MessageType::Info)
                            .buttons(gtk4::ButtonsType::Ok)
                            .text("Snapshot Przywrócony")
                            .secondary_text("Konfiguracja powróciła do stanu z wybranego punktu.")
                            .build();
                        dialog.connect_response(|d, _| d.close());
                        dialog.show();
                    }
                    Err(e) => {
                        let dialog = gtk4::MessageDialog::builder()
                            .transient_for(&win_clone)
                            .modal(true)
                            .message_type(gtk4::MessageType::Error)
                            .buttons(gtk4::ButtonsType::Close)
                            .text("Błąd przywracania")
                            .secondary_text(&e.to_string())
                            .build();
                        dialog.connect_response(|d, _| d.close());
                        dialog.show();
                    }
                }
            });

            let delete_btn = Button::builder()
                .label("Usuń")
                .valign(gtk4::Align::Center)
                .build();
            delete_btn.add_css_class("destructive-action");

            let snap_svc_delete = snapshot_svc.clone();
            let listbox_clone = listbox.clone();
            let win_delete = window.clone();

            delete_btn.connect_clicked(move |_| {
                log::info!("UI Action: 'Delete Snapshot' clicked for snapshot {}", snapshot_id);

                let result = snap_svc_delete
                    .delete_snapshot(&snapshot_id)
                    .and_then(|_| HistoryService::new()?.delete_entries_for_snapshot(&snapshot_id));

                match result {
                    Ok(_) => {
                        populate_history(&listbox_clone, snap_svc_delete.clone(), &win_delete);
                    }
                    Err(e) => {
                        let dialog = gtk4::MessageDialog::builder()
                            .transient_for(&win_delete)
                            .modal(true)
                            .message_type(gtk4::MessageType::Error)
                            .buttons(gtk4::ButtonsType::Close)
                            .text("Błąd usuwania snapshotu")
                            .secondary_text(&e.to_string())
                            .build();
                        dialog.connect_response(|d, _| d.close());
                        dialog.show();
                    }
                }
            });

            actions_box.append(&restore_btn);
            actions_box.append(&delete_btn);
            row_box.append(&actions_box);
        }

        row.set_child(Some(&row_box));
        listbox.append(&row);
    }
}

pub fn build(window: &gtk4::ApplicationWindow) -> gtk4::Widget {
    let container = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .spacing(15)
        .build();

    // WAŻNE: & w markup GTK musi być &amp;
    let title = Label::builder()
        .label("<b>Historia zmian &amp; Snapshoty</b>")
        .use_markup(true)
        .halign(gtk4::Align::Center)
        .build();

    let create_snap_btn = Button::builder()
        .label("📸  Stwórz Snapshot Teraz")
        .halign(gtk4::Align::Center)
        .build();
    create_snap_btn.add_css_class("suggested-action");

    container.append(&title);
    container.append(&create_snap_btn);

    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let listbox = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();
    listbox.add_css_class("boxed-list");

    let snapshot_svc = Rc::new(SnapshotService::new().expect("Failed to init SnapshotService"));

    // Załaduj historię przy starcie
    populate_history(&listbox, snapshot_svc.clone(), window);

    scrolled.set_child(Some(&listbox));
    container.append(&scrolled);

    // Przycisk Create Snapshot — po kliknięciu odświeża listę
    let snap_svc_clone = snapshot_svc.clone();
    let win_clone = window.clone();
    let listbox_clone = listbox.clone();
    let win_clone2 = window.clone();

    create_snap_btn.connect_clicked(move |_| {
        log::info!("UI Action: 'Create Snapshot Now' clicked");
        match snap_svc_clone.create_snapshot(domain::snapshot::SnapshotKind::Manual) {
            Ok(snapshot) => {
                // Zapisz do historii
                if let Ok(history_svc) = HistoryService::new() {
                    let _ = history_svc.log_change(domain::history::HistoryEntry::new(
                        ChangeKind::ManualSnapshot(snapshot.id),
                        None,
                        Some(snapshot.id),
                    ));
                }

                // Odśwież listę historii natychmiast!
                populate_history(&listbox_clone, snap_svc_clone.clone(), &win_clone);

                let dialog = gtk4::MessageDialog::builder()
                    .transient_for(&win_clone2)
                    .modal(true)
                    .message_type(gtk4::MessageType::Info)
                    .buttons(gtk4::ButtonsType::Ok)
                    .text("Snapshot Stworzony")
                    .secondary_text(&format!(
                        "Snapshot {} zapisany. Widoczny na liście powyżej.",
                        &snapshot.id.to_string()[..8]
                    ))
                    .build();
                dialog.connect_response(|d, _| d.close());
                dialog.show();
            }
            Err(e) => {
                let dialog = gtk4::MessageDialog::builder()
                    .transient_for(&win_clone2)
                    .modal(true)
                    .message_type(gtk4::MessageType::Error)
                    .buttons(gtk4::ButtonsType::Close)
                    .text("Błąd tworzenia snapshotu")
                    .secondary_text(&e.to_string())
                    .build();
                dialog.connect_response(|d, _| d.close());
                dialog.show();
            }
        }
    });

    container.upcast()
}
