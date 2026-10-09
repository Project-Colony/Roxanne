#![cfg(unix)]

use roxanne::file_ops::atomic_write;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn saving_keeps_the_file_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    // 0o600 is the mode the temporary file starts with, so 0o751 shows the
    // original mode is really copied.
    for mode in [0o600, 0o751] {
        let path = dir.path().join(format!("file-{mode:o}"));
        fs::write(&path, "old").expect("write file");
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("chmod");

        atomic_write(path.to_str().expect("path"), "new").expect("atomic write");

        assert_eq!(fs::read_to_string(&path).expect("read file"), "new");
        let saved = fs::metadata(&path).expect("metadata").permissions().mode() & 0o7777;
        assert_eq!(saved, mode, "mode changed from {mode:o} to {saved:o}");
    }
}

#[test]
fn saving_through_a_symlink_updates_its_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real_dir = dir.path().join("real");
    let link_dir = dir.path().join("links");
    fs::create_dir(&real_dir).expect("create real dir");
    fs::create_dir(&link_dir).expect("create link dir");
    let target = real_dir.join("config.toml");
    let link = link_dir.join("config.toml");
    fs::write(&target, "old").expect("write target");
    // A relative link into another directory: the temporary file must be
    // created next to the target, not next to the link.
    symlink("../real/config.toml", &link).expect("symlink");

    atomic_write(link.to_str().expect("path"), "new").expect("atomic write");

    let link_meta = fs::symlink_metadata(&link).expect("link metadata");
    assert!(link_meta.file_type().is_symlink(), "the link was replaced");
    assert_eq!(
        fs::read_link(&link).expect("read link"),
        std::path::Path::new("../real/config.toml")
    );
    assert_eq!(fs::read_to_string(&target).expect("read target"), "new");
    for dir in [&real_dir, &link_dir] {
        let names: Vec<_> = fs::read_dir(dir)
            .expect("read dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(
            names.len(),
            1,
            "leftover files in {}: {names:?}",
            dir.display()
        );
    }
}
