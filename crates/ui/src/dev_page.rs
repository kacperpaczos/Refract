use app::adapter_snapshot_service::GnomeAdapterSnapshotService;
use app::export_service::ExportService;
use gtk4::{prelude::*, Align, Box, Button, Label, Orientation};

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

fn build_tool_card(title: &str, description: &str, button: &Button) -> Box {
    let card = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .hexpand(true)
        .build();
    card.add_css_class("boxed-list");

    let title_label = Label::builder()
        .label(title)
        .halign(Align::Start)
        .wrap(true)
        .build();
    title_label.add_css_class("heading");

    let description_label = Label::builder()
        .label(description)
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .build();
    description_label.add_css_class("dim-label");

    button.set_halign(Align::Start);

    card.append(&title_label);
    card.append(&description_label);
    card.append(button);
    card
}

pub fn build(window: &gtk4::ApplicationWindow) -> gtk4::Widget {
    let container = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .spacing(16)
        .build();

    let title = Label::builder()
        .label("<b>Dev Tools</b>")
        .use_markup(true)
        .halign(Align::Center)
        .build();

    let subtitle = Label::builder()
        .label("Narzędzia pomocnicze do eksportu, debugowania i dalszej pracy nad layoutami.")
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .halign(Align::Center)
        .build();
    subtitle.add_css_class("dim-label");

    let status_label = Label::builder()
        .label("")
        .wrap(true)
        .halign(Align::Start)
        .xalign(0.0)
        .build();
    status_label.add_css_class("dim-label");

    let export_btn = Button::builder()
        .label("Export Current GNOME Layout")
        .build();

    let export_window = window.clone();
    let export_status = status_label.clone();
    export_btn.connect_clicked(move |_| {
        log::info!("UI Action: 'Export Current GNOME Layout' clicked");
        match ExportService::export_current_gnome_layout() {
            Ok(result) => {
                export_status.set_label(
                    "Eksport gotowy. Utworzono katalog eksportu i startowy plik .de w katalogu user layouts.",
                );
                show_info_dialog(
                    &export_window,
                    "GNOME Layout Exported",
                    &format!(
                        "Export dir:\n{}\n\nEditable user layout file:\n{}\n\nRestart the app after editing the .de file and it should appear in the layouts list.",
                        result.export_dir.to_string_lossy(),
                        result.user_layout_file.to_string_lossy()
                    ),
                );
            }
            Err(error) => {
                export_status.set_label("Nie udało się wyeksportować bieżącego układu GNOME.");
                show_error_dialog(&export_window, "GNOME Export Failed", &error.to_string());
            }
        }
    });

    let adapter_snapshot_btn = Button::builder()
        .label("Snapshot GNOME Adapter")
        .build();

    let snapshot_window = window.clone();
    let snapshot_status = status_label.clone();
    adapter_snapshot_btn.connect_clicked(move |_| {
        log::info!("UI Action: 'Snapshot GNOME Adapter' clicked");
        match GnomeAdapterSnapshotService::create_snapshot() {
            Ok(result) => {
                snapshot_status.set_label(
                    "Snapshot adaptera zapisany. To pojedynczy plik JSON do debugowania i analizy stanu systemu.",
                );
                show_info_dialog(
                    &snapshot_window,
                    "GNOME Adapter Snapshot Saved",
                    &format!(
                        "Snapshot file:\n{}\n\nThis JSON contains platform, session, tools, extensions, dconf and current layout engine state.",
                        result.output_file.to_string_lossy()
                    ),
                );
            }
            Err(error) => {
                snapshot_status.set_label("Nie udało się utworzyć snapshotu adaptera GNOME.");
                show_error_dialog(&snapshot_window, "GNOME Adapter Snapshot Failed", &error.to_string());
            }
        }
    });

    let export_card = build_tool_card(
        "Export layoutu",
        "Zapisuje bieżący układ GNOME do katalogu eksportu i tworzy edytowalny plik .de, z którego można zrobić własny layout w aplikacji.",
        &export_btn,
    );

    let snapshot_card = build_tool_card(
        "Snapshot adaptera",
        "Zbiera dane diagnostyczne o sesji GNOME, narzędziach, rozszerzeniach i dconf do jednego pliku JSON. Przydaje się do debugowania.",
        &adapter_snapshot_btn,
    );

    container.append(&title);
    container.append(&subtitle);
    container.append(&export_card);
    container.append(&snapshot_card);
    container.append(&status_label);

    container.upcast()
}
