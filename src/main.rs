// A GUI program: no console window opens next to the editor on Windows.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> iced::Result {
    // Answered before the config is read and before any window opens, so it
    // works headless: the release workflow smoke-tests every binary with it.
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--version" || arg == "-V")
    {
        println!("roxanne {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    roxanne::ui::run()
}
