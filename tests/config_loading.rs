use iced::keyboard;
use roxanne::config::AppConfig;
use roxanne::keymap::{KeyAction, KeymapMode};
use roxanne::theme::ThemePalette;
use std::fs;
use std::path::{Path, PathBuf};

struct EnvGuard {
    original_home: Option<String>,
    original_dir: PathBuf,
}

impl EnvGuard {
    fn new(home: &Path, dir: &Path) -> Self {
        let original_home = std::env::var("HOME").ok();
        let original_dir = std::env::current_dir().expect("current dir");
        unsafe {
            std::env::set_var("HOME", home);
        }
        std::env::set_current_dir(dir).expect("set current dir");
        Self {
            original_home,
            original_dir,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(home) = &self.original_home {
            unsafe {
                std::env::set_var("HOME", home);
            }
        }
        std::env::set_current_dir(&self.original_dir).expect("restore current dir");
    }
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write file");
}

#[test]
fn loads_profile_and_workspace_overrides() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).expect("create workspace");
    let _guard = EnvGuard::new(&home, &workspace);

    let user_config = home.join(".config/roxanne/config.toml");
    write_file(
        &user_config,
        r#"
profile = "dev"

[theme]
name = "light"
"#,
    );

    let profile_config = home.join(".config/roxanne/profiles/dev.toml");
    write_file(
        &profile_config,
        r#"
[keymap]
save = "ctrl+shift+s"
"#,
    );

    let workspace_config = workspace.join(".roxanne.toml");
    write_file(
        &workspace_config,
        r#"
keymap_profile = "workspace"

[theme]
name = "dark"

[keymap_profiles.workspace]
save = "alt+s"
"#,
    );

    let config = AppConfig::load();

    let dark = ThemePalette::from_name("dark").expect("dark theme");
    assert_eq!(config.theme.app_background, dark.app_background);

    let key = keyboard::Key::Character("s".into());
    let save_action = config
        .keymap
        .match_event(&key, keyboard::Modifiers::ALT, KeymapMode::Insert);
    assert_eq!(save_action, Some(KeyAction::Save));

    let ctrl_shift = keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT;
    let old_action = config
        .keymap
        .match_event(&key, ctrl_shift, KeymapMode::Insert);
    assert_ne!(old_action, Some(KeyAction::Save));
}
