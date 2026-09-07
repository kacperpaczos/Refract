use gtk4::{prelude::*, Application, ApplicationWindow, HeaderBar, Stack, StackSwitcher};

use crate::{dev_page, history_page, layout_page, setup_page};

pub fn build(app: &Application) -> ApplicationWindow {
    log::info!("window::build(): begin");
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Desktop Experience Switcher")
        .default_width(700)
        .default_height(600)
        .build();

    let header_bar = HeaderBar::builder().show_title_buttons(true).build();
    window.set_titlebar(Some(&header_bar));

    // Ekran inicjalizacji — pokazuje się jako pierwszy
    let win_clone = window.clone();
    let header_clone = header_bar.clone();

    let setup = setup_page::build(&window, move || {
        log::info!("window::build(): setup completed, switching to main stack");
        // on_ready: podmień zawartość okna na główny interfejs
        let stack = Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .build();

        let stack_switcher = StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk4::Align::Center)
            .build();
        header_clone.set_title_widget(Some(&stack_switcher));

        let layout_page = layout_page::build(&win_clone);
        stack.add_titled(&layout_page, Some("layouts"), "Layouts");

        let history_page = history_page::build(&win_clone);
        stack.add_titled(&history_page, Some("history"), "Historia");

        let dev_page = dev_page::build(&win_clone);
        stack.add_titled(&dev_page, Some("dev"), "Dev");

        win_clone.set_child(Some(&stack));
    });

    window.set_child(Some(&setup));

    log::info!("window::build(): done");
    window
}
