use std::fs::{File, Metadata, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

pub const MAX_OPEN_FILE_SIZE: u64 = 5 * 1024 * 1024;

#[cfg(test)]
fn rename_failure_state() -> &'static Mutex<Option<PathBuf>> {
    static RENAME_FAILURE_TARGET: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    RENAME_FAILURE_TARGET.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub fn set_rename_failure_target(path: Option<PathBuf>) {
    let mut guard = rename_failure_state().lock().expect("rename failure lock");
    *guard = path;
}

fn rename_file(from: &Path, to: &Path) -> Result<(), std::io::Error> {
    #[cfg(test)]
    {
        let mut guard = rename_failure_state().lock().expect("rename failure lock");
        if let Some(target) = guard.as_ref()
            && target.as_path() == to
        {
            *guard = None;
            return Err(std::io::Error::other("simulated rename failure"));
        }
    }

    std::fs::rename(from, to)
}

/// Replaces the file at `path` with `contents`, so it never holds a partial write.
///
/// The text goes to a temporary file next to the real file (a symlink is
/// followed, so the link stays and its target gets the new text), which then
/// takes the file's place in one rename. The new file keeps the old one's
/// permissions and, on Unix, its owner and group as far as the user may set them.
/// On any failure only the temporary file is removed and the original is left
/// as it was.
///
/// The rename gives the file a new inode, so other hard links to it keep the
/// old text. An in-place write would keep them, but could leave a half-written
/// file behind.
pub fn atomic_write(path: &str, contents: &str) -> Result<(), String> {
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    let (path, original) = match std::fs::metadata(path) {
        // Some volumes cannot resolve a path (Windows RAM disks and some
        // user-space file systems): save to the path as given there.
        Ok(metadata) => (
            std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path)),
            Some(metadata),
        ),
        Err(err) if err.kind() == ErrorKind::NotFound => (PathBuf::from(path), None),
        Err(err) => return Err(err.to_string()),
    };
    let parent = path.parent().unwrap_or(Path::new(""));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("roxanne");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_millis();
    let base_name = format!(".{file_name}.{stamp}");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if original.is_some() {
        use std::os::unix::fs::OpenOptionsExt;
        // Owner-only until the original permissions are copied, so no other
        // user can open the temporary file and read the new text.
        options.mode(0o600);
    }
    let mut attempts = 0_u32;
    let (temp_path, temp_file) = loop {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!("{base_name}.{counter}.tmp");
        let temp_path = if parent.as_os_str().is_empty() {
            PathBuf::from(&temp_name)
        } else {
            parent.join(&temp_name)
        };
        match options.open(&temp_path) {
            Ok(file) => break (temp_path, file),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                attempts += 1;
                if attempts > 1000 {
                    return Err("impossible de créer un fichier temporaire unique".to_string());
                }
            }
            Err(err) => return Err(err.to_string()),
        }
    };

    // A rename replaces the target in one step (MoveFileEx with
    // MOVEFILE_REPLACE_EXISTING on Windows), so the original is never removed
    // before the new text is in place.
    fill_temp_file(temp_file, contents, original.as_ref())
        .and_then(|()| rename_file(&temp_path, &path))
        .map_err(|err| {
            let _ = std::fs::remove_file(&temp_path);
            err.to_string()
        })
}

fn fill_temp_file(
    mut file: File,
    contents: &str,
    original: Option<&Metadata>,
) -> std::io::Result<()> {
    file.write_all(contents.as_bytes())?;
    if let Some(original) = original {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, fchown};
            // Only root may give a file away, but any owner may set a group
            // they belong to, so fall back to the group alone.
            let _ = fchown(&file, Some(original.uid()), Some(original.gid()))
                .or_else(|_| fchown(&file, None, Some(original.gid())));
        }
        // After the chown, which can clear the setuid and setgid bits.
        file.set_permissions(original.permissions())?;
    }
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn build_text(lines: usize, line_len: usize) -> String {
        let line = "a".repeat(line_len);
        std::iter::repeat_n(line, lines)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn atomic_write_writes_contents_without_temp_leftover() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("note.txt");
        atomic_write(path.to_str().expect("path"), "hello").expect("atomic write");

        let contents = fs::read_to_string(&path).expect("read file");
        assert_eq!(contents, "hello");

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.ends_with(".tmp"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary files were not cleaned up: {leftovers:?}"
        );
    }

    #[test]
    fn atomic_write_handles_large_payloads() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("large.txt");
        let payload = build_text(50_000, 80);

        atomic_write(path.to_str().expect("path"), &payload).expect("atomic write");
        let contents = fs::read_to_string(&path).expect("read file");

        assert_eq!(contents.len(), payload.len());
        assert_eq!(contents, payload);
    }

    #[test]
    fn atomic_write_preserves_original_on_rename_failure() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("note.txt");
        fs::write(&path, "original").expect("write original");

        // atomic_write renames onto the resolved path (macOS tempdirs sit
        // behind the /var symlink).
        set_rename_failure_target(Some(fs::canonicalize(&path).expect("canonical path")));
        let result = atomic_write(path.to_str().expect("path"), "updated");
        assert!(result.is_err(), "atomic write should fail");

        let contents = fs::read_to_string(&path).expect("read original");
        assert_eq!(contents, "original");

        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.ends_with(".tmp") || name.ends_with(".bak"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary files were not cleaned up: {leftovers:?}"
        );
    }
}
