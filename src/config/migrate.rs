//! The one-time copy of the settings Roxanne kept in `~/.config/roxanne`
//! before it moved to `Colony/Roxanne`.
//!
//! The old directory is copied, never moved or deleted: if the copy turns out
//! wrong, the user's files are still where they were. A marker file in the new
//! directory stops the copy from running twice.

use super::user_config_dir;
use crate::i18n::t_fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Written in the new directory once the old one has been copied.
const MARKER: &str = ".migrated-from-legacy-config";

/// Copies `config.toml` and `profiles/` from the legacy directory into
/// `Colony/Roxanne` when that has no `config.toml` yet. Returns a message for
/// the status bar when something was copied or the copy failed.
pub fn migrate_legacy_config() -> Option<String> {
    let legacy =
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/roxanne"))?;
    let new = user_config_dir()?;
    match migrate(&legacy, &new) {
        Ok(false) => None,
        Ok(true) => Some(t_fmt(
            "status_config_migrated",
            &[("from", &legacy.display()), ("to", &new.display())],
        )),
        Err(err) => Some(t_fmt(
            "status_config_migration_failed",
            &[("from", &legacy.display()), ("error", &err)],
        )),
    }
}

/// Copies `legacy` into `new` unless the marker says it was done, the legacy
/// directory is absent, or `new` already has a `config.toml`. Returns whether
/// files were copied. On an error the marker is not written, so the next start
/// tries again; the legacy files are never touched.
fn migrate(legacy: &Path, new: &Path) -> io::Result<bool> {
    if !legacy.is_dir() || new.join(MARKER).exists() {
        return Ok(false);
    }
    if new.join("config.toml").exists() {
        // The user already has settings here: never overwrite them.
        write_marker(new)?;
        return Ok(false);
    }
    fs::create_dir_all(new)?;
    // Profiles first: config.toml appearing in the new directory is what ends
    // the migration, so it is copied last.
    let profiles = legacy.join("profiles");
    if profiles.is_dir() {
        let target = new.join("profiles");
        fs::create_dir_all(&target)?;
        for entry in fs::read_dir(&profiles)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(entry.path(), target.join(entry.file_name()))?;
            }
        }
    }
    let config = legacy.join("config.toml");
    if config.is_file() {
        fs::copy(&config, new.join("config.toml"))?;
    }
    write_marker(new)?;
    Ok(true)
}

fn write_marker(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(MARKER),
        "Roxanne copied its settings here from ~/.config/roxanne once.\n\
         Delete this file to copy them again when config.toml is missing.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_with_settings(root: &Path) -> PathBuf {
        let legacy = root.join("legacy");
        fs::create_dir_all(legacy.join("profiles")).expect("profiles");
        fs::write(legacy.join("config.toml"), "# mine\nprofile = \"work\"\n").expect("config");
        fs::write(
            legacy.join("profiles/work.toml"),
            "[editor]\ntab_size = 2\n",
        )
        .expect("profile");
        legacy
    }

    #[test]
    fn copies_the_settings_once_and_keeps_the_originals() {
        let root = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_with_settings(root.path());
        let new = root.path().join("Colony/Roxanne");

        assert!(migrate(&legacy, &new).expect("migrate"));
        assert_eq!(
            fs::read_to_string(new.join("config.toml")).expect("config"),
            "# mine\nprofile = \"work\"\n"
        );
        assert!(new.join("profiles/work.toml").is_file());
        assert!(new.join(MARKER).is_file());
        assert!(
            legacy.join("config.toml").is_file(),
            "the original was removed"
        );

        // A second start changes nothing, even after the user edits the copy.
        fs::write(new.join("config.toml"), "language = \"fr\"\n").expect("edit");
        fs::remove_file(new.join("profiles/work.toml")).expect("remove");
        assert!(!migrate(&legacy, &new).expect("second run"));
        assert_eq!(
            fs::read_to_string(new.join("config.toml")).expect("config"),
            "language = \"fr\"\n"
        );
        assert!(!new.join("profiles/work.toml").exists());
    }

    #[test]
    fn never_overwrites_an_existing_config() {
        let root = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_with_settings(root.path());
        let new = root.path().join("Colony/Roxanne");
        fs::create_dir_all(&new).expect("new dir");
        fs::write(new.join("config.toml"), "language = \"en\"\n").expect("config");

        assert!(!migrate(&legacy, &new).expect("migrate"));
        assert_eq!(
            fs::read_to_string(new.join("config.toml")).expect("config"),
            "language = \"en\"\n"
        );
        assert!(!new.join("profiles").exists());
    }

    #[test]
    fn does_nothing_without_a_legacy_directory() {
        let root = tempfile::tempdir().expect("tempdir");
        let new = root.path().join("Colony/Roxanne");
        assert!(!migrate(&root.path().join("legacy"), &new).expect("migrate"));
        assert!(!new.exists());
    }
}
