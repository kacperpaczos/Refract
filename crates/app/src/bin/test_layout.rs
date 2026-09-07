use app::layout_service::LayoutService;
use std::path::Path;

fn main() {
    println!("Init test...");


    std::env::set_var("DESKTOP_EXPERIENCE_DATA_DIR", "/home/kacper/Desktop/gnome-sriczer/desktop-experience-switcher/data");
    
    log::info!("Calling LayoutService::new()...");
    let svc = match LayoutService::new() {
        Ok(s) => s,
        Err(e) => {
            log::error!("err: {}", e);
            return;
        }
    };

    log::info!("Loading layouts...");
    let layouts = app::layout_service::LayoutService::load_layouts(Path::new("/home/kacper/Desktop/gnome-sriczer/desktop-experience-switcher/data"))
        .expect("failed to load");
        
    let traditional = layouts.iter().find(|l| l.id == "traditional").expect("Not found");
    
    log::info!("Applying layout...");
    match svc.apply_layout(traditional) {
        Ok(report) if report.is_success() => {
            log::info!("Apply layout finished successfully: {:?}", report.status);
        }
        Ok(report) => {
            log::error!("Apply layout finished with status {:?}: {}", report.status, report.detail);
        }
        Err(e) => {
            log::error!("Apply layout failed: {}", e);
        }
    }
    
    log::info!("Done.");
}
