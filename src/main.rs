mod app;
mod config;
mod editor;
mod keymap;
mod plugins;
mod theme;

fn main() -> iced::Result {
    let config = config::AppConfig::load();
    app::RoxanneApp::run_with_config(config)
}
