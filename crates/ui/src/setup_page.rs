use gtk4::{prelude::*, Box, Button, Label, Orientation};
use infra::tools::{ensure_tool, is_installed, TOOL_GEXT};

/// Ekran startowy sprawdzający narzędzia.
/// on_ready wywoływany gdy wszystko gotowe lub użytkownik chce kontynuować.
pub fn build(_window: &gtk4::ApplicationWindow, on_ready: impl Fn() + 'static) -> gtk4::Widget {
    let container = Box::builder()
        .orientation(Orientation::Vertical)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .spacing(20)
        .margin_top(60)
        .margin_bottom(60)
        .margin_start(60)
        .margin_end(60)
        .build();

    let title = Label::builder()
        .label("<b>Desktop Experience Switcher</b>")
        .use_markup(true)
        .build();

    let status_lbl = Label::builder()
        .label("⏳ Sprawdzanie narzędzi...")
        .halign(gtk4::Align::Center)
        .wrap(true)
        .max_width_chars(55)
        .build();

    let start_btn = Button::builder()
        .label("Przejdź do aplikacji ▶")
        .halign(gtk4::Align::Center)
        .sensitive(false)
        .build();
    start_btn.add_css_class("suggested-action");

    let install_btn = Button::builder()
        .label("Zainstaluj gext i kontynuuj")
        .halign(gtk4::Align::Center)
        .visible(false)
        .build();

    container.append(&title);
    container.append(&status_lbl);
    container.append(&start_btn);
    container.append(&install_btn);

    // --- Callback "Przejdź" ---
    let on_ready = std::rc::Rc::new(on_ready);
    let on_ready_btn = on_ready.clone();
    start_btn.connect_clicked(move |_| { (on_ready_btn)(); });

    // --- Callback "Zainstaluj" ---
    let status_install = status_lbl.clone();
    let start_install = start_btn.clone();
    let install_hidden = install_btn.clone();
    let on_ready_install = on_ready.clone();

    install_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        status_install.set_label("⏳ Instalowanie... Pojawi się okno polkit.");

        let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::spawn(move || {
            let res = ensure_tool(&TOOL_GEXT).map_err(|e| e.to_string());
            let _ = tx.send(res);
        });

        let status_c = status_install.clone();
        let start_c = start_install.clone();
        let install_c = install_hidden.clone();
        let on_ready_c = on_ready_install.clone();

        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    status_c.set_label("✅ gext zainstalowany poprawnie!");
                    start_c.set_sensitive(true);
                    install_c.set_visible(false);
                    // Auto-przejście
                    (on_ready_c)();
                    gtk4::glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    status_c.set_label(&format!("⚠️  {}\n\nMożesz kontynuować bez pełnej obsługi wtyczek.", e));
                    start_c.set_label("Kontynuuj bez gext ▶");
                    start_c.set_sensitive(true);
                    install_c.set_sensitive(true);
                    gtk4::glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
                Err(_) => gtk4::glib::ControlFlow::Break,
            }
        });
    });

    // --- Sprawdzenie synchroniczne po pokazaniu okna (timeout = 0ms → następna rama UI) ---
    let status_check = status_lbl.clone();
    let start_check = start_btn.clone();
    let install_check = install_btn.clone();

    gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
        if is_installed(&TOOL_GEXT) {
            status_check.set_label("✅ Wszystkie narzędzia gotowe.");
            start_check.set_sensitive(true);
        } else {
            status_check.set_label(
                "⚠️  Narzędzie gext nie jest zainstalowane.\n\
                 Musisz je zainstalować, aby móc instalować rozszerzenia GNOME.",
            );
            install_check.set_visible(true);
            // Pozwól też kontynuować bez gext (ograniczona funkcja)
            start_check.set_label("Kontynuuj bez gext ▶");
            start_check.set_sensitive(true);
            start_check.remove_css_class("suggested-action");
        }
    });

    container.upcast()
}
