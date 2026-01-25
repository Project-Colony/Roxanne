use roxanne::{app, config};

fn main() -> iced::Result {
    let config = config::AppConfig::load();
    app::RoxanneApp::run_with_config(config)
}
