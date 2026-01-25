use roxanne::config::migrate_config;
use toml::Value;

fn migrate(contents: &str) -> Value {
    let raw: Value = toml::from_str(contents).expect("parse toml");
    migrate_config(raw)
}

#[test]
fn migrates_keybindings_to_keymap_and_sets_version() {
    let migrated = migrate(
        r#"
[keybindings]
save = "ctrl+s"
find = "ctrl+f"
"#,
    );

    let version = migrated
        .get("config_version")
        .and_then(Value::as_integer);
    assert_eq!(version, Some(1));

    let keymap = migrated
        .get("keymap")
        .and_then(Value::as_table)
        .expect("keymap table");
    assert_eq!(keymap.get("save").and_then(Value::as_str), Some("ctrl+s"));
    assert_eq!(keymap.get("find").and_then(Value::as_str), Some("ctrl+f"));

    assert!(migrated.get("keybindings").is_none());
}

#[test]
fn preserves_existing_keymap_when_merging_keybindings() {
    let migrated = migrate(
        r#"
config_version = 0

[keymap]
save = "cmd+s"

[keybindings]
save = "ctrl+s"
find = "ctrl+f"
"#,
    );

    let keymap = migrated
        .get("keymap")
        .and_then(Value::as_table)
        .expect("keymap table");
    assert_eq!(keymap.get("save").and_then(Value::as_str), Some("cmd+s"));
    assert_eq!(keymap.get("find").and_then(Value::as_str), Some("ctrl+f"));
}
