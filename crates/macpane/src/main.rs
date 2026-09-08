#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod input;
mod session;
mod ui;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MacPane")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MacPane",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}
