use iced::keyboard;
use roxanne::config::AppConfig;
use roxanne::keymap::{KeyAction, KeymapMode};
use roxanne::theme::ThemePalette;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

// HOME and the working directory are process-wide, and the test harness runs
// tests on parallel threads: every test that changes them holds this lock.
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    original_home: Option<String>,
    original_dir: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn new(home: &Path, dir: &Path) -> Self {
        let lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let original_home = std::env::var("HOME").ok();
        let original_dir = std::env::current_dir().expect("current dir");
        unsafe {
            std::env::set_var("HOME", home);
        }
        std::env::set_current_dir(dir).expect("set current dir");
        Self {
            original_home,
            original_dir,
            _lock: lock,
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

#[test]
fn loads_editor_config_from_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).expect("create workspace");
    let _guard = EnvGuard::new(&home, &workspace);

    let workspace_config = workspace.join(".roxanne.toml");
    write_file(
        &workspace_config,
        r#"
[editor]
tab_size = 2
use_spaces = false
line_ending = "crlf"
word_wrap = true
minimap = true
"#,
    );

    let config = AppConfig::load();
    assert_eq!(config.editor.tab_size, 2);
    assert!(!config.editor.use_spaces);
    assert_eq!(config.editor.line_ending, "crlf");
    assert!(config.editor.word_wrap);
    assert!(config.editor.minimap);
}

#[test]
fn editor_config_defaults() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).expect("create workspace");
    let _guard = EnvGuard::new(&home, &workspace);

    let config = AppConfig::load();
    assert_eq!(config.editor.tab_size, 4);
    assert!(config.editor.use_spaces);
    assert_eq!(config.editor.line_ending, "lf");
    assert!(!config.editor.word_wrap);
    assert!(!config.editor.minimap);
}

#[test]
fn project_config_never_lists_native_plugins() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    let nested = project.join("src/deep");
    fs::create_dir_all(&nested).expect("create project");
    // Started from a sub-directory, so the project file is found in an ancestor.
    let _guard = EnvGuard::new(&home, &nested);

    write_file(
        &home.join(".config/roxanne/config.toml"),
        r#"
[plugins]
dynamic = ["/opt/roxanne/libtrusted.so"]
"#,
    );
    write_file(
        &home.join(".config/roxanne/profiles/rogue.toml"),
        r#"
[plugins]
dynamic = ["/opt/roxanne/libprofile.so"]
"#,
    );
    write_file(
        &project.join(".roxanne.toml"),
        r#"
profile = "rogue"

[plugins]
dynamic = ["/tmp/libevil.so"]
"#,
    );

    let config = AppConfig::load();
    assert_eq!(config.plugins.dynamic, ["/opt/roxanne/libtrusted.so"]);
    assert!(
        config
            .load_warnings
            .iter()
            .any(|warning| warning.contains("plugins.dynamic"))
    );
}
