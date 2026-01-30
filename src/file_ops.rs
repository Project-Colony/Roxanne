use std::fs::OpenOptions;
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
        if let Some(target) = guard.as_ref() {
            if target.as_path() == to {
                *guard = None;
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    "simulated rename failure",
                ));
            }
        }
    }

    std::fs::rename(from, to)
}

pub fn atomic_write(path: &str, contents: &str) -> Result<(), String> {
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    let path = Path::new(path);
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
    let mut attempts = 0_u32;
    let (temp_path, mut temp_file) = loop {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!("{base_name}.{counter}.tmp");
        let temp_path = if parent.as_os_str().is_empty() {
            PathBuf::from(&temp_name)
        } else {
            parent.join(&temp_name)
        };
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
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
    temp_file
        .write_all(contents.as_bytes())
        .and_then(|_| temp_file.sync_all())
        .map_err(|err| err.to_string())?;
    drop(temp_file);

    let backup_path = if path.exists() {
        let mut attempts = 0_u32;
        loop {
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let backup_name = format!("{base_name}.{counter}.bak");
            let candidate = if parent.as_os_str().is_empty() {
                PathBuf::from(&backup_name)
            } else {
                parent.join(&backup_name)
            };
            if !candidate.exists() {
                break Some(candidate);
            }
            attempts += 1;
            if attempts > 1000 {
                let _ = std::fs::remove_file(&temp_path);
                return Err("impossible de créer un fichier de sauvegarde unique".to_string());
            }
        }
    } else {
        None
    };

    if let Some(backup_path) = backup_path.as_ref() {
        if let Err(err) = rename_file(path, backup_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(err.to_string());
        }
    }

    match rename_file(&temp_path, path) {
        Ok(()) => {
            if let Some(backup_path) = backup_path {
                if let Err(err) = std::fs::remove_file(&backup_path) {
                    return Err(format!("suppression sauvegarde échouée: {err}"));
                }
            }
            Ok(())
        }
        Err(err) => {
            let _ = std::fs::remove_file(&temp_path);
            if let Some(backup_path) = backup_path {
                if let Err(restore_err) = rename_file(&backup_path, path) {
                    let _ = std::fs::remove_file(&backup_path);
                    return Err(format!(
                        "{err} (restauration échouée: {restore_err})"
                    ));
                }
            }
            Err(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    /// Tests that use the global rename failure target must be serialized.
    static RENAME_TEST_LOCK: std::sync::LazyLock<Mutex<()>> =
        std::sync::LazyLock::new(|| Mutex::new(()));

    fn build_text(lines: usize, line_len: usize) -> String {
        let line = "a".repeat(line_len);
        std::iter::repeat(line)
            .take(lines)
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
        let _lock = RENAME_TEST_LOCK.lock().unwrap();
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("note.txt");
        fs::write(&path, "original").expect("write original");

        set_rename_failure_target(Some(path.clone()));
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

    #[test]
    fn atomic_write_restores_backup_after_failed_rename() {
        let _lock = RENAME_TEST_LOCK.lock().unwrap();
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("report.txt");
        fs::write(&path, "baseline").expect("write original");

        set_rename_failure_target(Some(path.clone()));
        let result = atomic_write(path.to_str().expect("path"), "replacement");
        assert!(result.is_err(), "atomic write should fail");

        let contents = fs::read_to_string(&path).expect("read original");
        assert_eq!(contents, "baseline");

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
