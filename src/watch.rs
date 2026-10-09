//! Watches files for changes made by other programs.
//!
//! A watcher watches the directories that hold the files, not the files
//! themselves. Many programs save by writing a new file and renaming it over
//! the old one, which ends a watch on the old file. A directory watch keeps
//! working through such saves, and it also sees a file created after the watch
//! started.

use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::Metadata;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// The files Roxanne saved, with the modification time and length of each one.
static OWN_WRITES: Mutex<BTreeMap<PathBuf, (SystemTime, u64)>> = Mutex::new(BTreeMap::new());

/// Watches `paths` and sends `on_change(path)` for each one that changes.
///
/// A path named `*.toml` stands for every TOML file in its directory. `paths`
/// is called once, when the subscription starts, so `id` must change whenever
/// the paths do. A directory that does not exist yet is watched from its
/// parent until it is created. With `ignore_own_writes`, a file that is still
/// the one Roxanne last saved is not reported.
pub fn subscription<I, M>(
    id: I,
    paths: impl FnOnce() -> Vec<PathBuf> + Send + 'static,
    ignore_own_writes: bool,
    on_change: fn(PathBuf) -> M,
) -> Subscription<M>
where
    I: Hash + 'static,
    M: Send + 'static,
{
    iced::subscription::channel(id, 16, move |mut output| async move {
        let (sender, mut events) = mpsc::unbounded();
        let Ok(mut watcher) = notify::recommended_watcher(move |event| {
            let _ = sender.unbounded_send(event);
        }) else {
            loop {
                std::future::pending::<()>().await;
            }
        };

        let wanted: Vec<PathBuf> = paths().iter().map(|path| normalize(path)).collect();
        let mut dirs: Vec<&Path> = wanted.iter().filter_map(|path| path.parent()).collect();
        dirs.sort();
        dirs.dedup();
        let mut watched = Vec::new();
        let mut missing = Vec::new();
        for dir in dirs {
            if watcher.watch(dir, RecursiveMode::NonRecursive).is_ok() {
                watched.push(dir);
            } else {
                if let Some(parent) = dir.parent()
                    && !watched.contains(&parent)
                {
                    let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
                }
                missing.push(dir.to_path_buf());
            }
        }

        while let Some(event) = events.next().await {
            // One save often raises several events: report each file once.
            let mut changed = Vec::new();
            let mut next = Some(event);
            while let Some(event) = next {
                for path in event.as_ref().map(changed_paths).unwrap_or_default() {
                    if missing.contains(path) {
                        // A missing directory was created, maybe with another
                        // one below it: watch every one that exists now. Their
                        // files may have been written before the watch started.
                        missing.retain(|dir| {
                            let found = watcher.watch(dir, RecursiveMode::NonRecursive).is_ok();
                            if found {
                                changed.extend(
                                    wanted
                                        .iter()
                                        .filter(|file| file.parent() == Some(dir.as_path()))
                                        .cloned(),
                                );
                            }
                            !found
                        });
                    } else if reports(&wanted, path, ignore_own_writes) {
                        changed.push(path.clone());
                    }
                }
                next = events.try_next().ok().flatten();
            }
            changed.sort();
            changed.dedup();
            for path in changed {
                let _ = output.send(on_change(path)).await;
            }
        }
        // The watcher holds the sender, so the events never end.
        loop {
            std::future::pending::<()>().await;
        }
    })
}

/// Notes that the file Roxanne saved at `path` has the metadata `written`, so
/// watchers that ignore Roxanne's own saves skip it.
pub fn record_own_write(path: &Path, written: &Metadata) {
    if let (Ok(mut writes), Ok(modified)) = (OWN_WRITES.lock(), written.modified()) {
        writes.insert(normalize(path), (modified, written.len()));
    }
}

/// `path` as a watcher reports it: absolute and with symlinks resolved, or for
/// a file that does not exist, with its directory resolved.
pub fn normalize(path: &Path) -> PathBuf {
    if let Ok(path) = std::fs::canonicalize(path) {
        return path;
    }
    path.parent()
        .zip(path.file_name())
        .and_then(|(dir, name)| {
            let dir = if dir.as_os_str().is_empty() {
                Path::new(".")
            } else {
                dir
            };
            Some(std::fs::canonicalize(dir).ok()?.join(name))
        })
        .or_else(|| std::path::absolute(path).ok())
        .unwrap_or_else(|| path.to_path_buf())
}

/// The paths whose content an event may have changed. A rename counts for the
/// new name only, and a metadata change (permissions, times) not at all.
fn changed_paths(event: &Event) -> &[PathBuf] {
    match event.kind {
        EventKind::Create(_) => &event.paths,
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            event.paths.get(1..).unwrap_or_default()
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From) | ModifyKind::Metadata(_)) => &[],
        EventKind::Modify(_) => &event.paths,
        _ => &[],
    }
}

/// Whether a change to `path` is reported: it is one of the `wanted` paths
/// (where `dir/*.toml` stands for every TOML file in `dir`), and with
/// `ignore_own_writes`, not the file Roxanne last saved there.
fn reports(wanted: &[PathBuf], path: &Path, ignore_own_writes: bool) -> bool {
    let is_wanted = wanted.iter().any(|wanted| {
        wanted == path
            || (wanted.file_name() == Some(OsStr::new("*.toml"))
                && wanted.parent() == path.parent()
                && path.extension() == Some(OsStr::new("toml")))
    });
    is_wanted && !(ignore_own_writes && is_own_write(path))
}

/// Whether the file at `path` is still the one Roxanne last saved there.
fn is_own_write(path: &Path) -> bool {
    let Ok(writes) = OWN_WRITES.lock() else {
        return false;
    };
    writes.get(path).is_some_and(|&(modified, len)| {
        std::fs::metadata(path)
            .is_ok_and(|now| now.len() == len && now.modified().is_ok_and(|now| now == modified))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_ops::atomic_write;
    use notify::event::{CreateKind, DataChange, MetadataKind};

    fn event(kind: EventKind, paths: &[&Path]) -> Event {
        paths.iter().fold(Event::new(kind), |event, path| {
            event.add_path(path.to_path_buf())
        })
    }

    #[test]
    fn reports_only_the_watched_files_of_a_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dir = normalize(dir.path());
        let file = dir.join("notes.txt");
        let profiles = dir.join("profiles");
        let wanted = [file.clone(), profiles.join("*.toml")];
        let data = EventKind::Modify(ModifyKind::Data(DataChange::Any));

        let reported = |event: &Event| -> Vec<PathBuf> {
            changed_paths(event)
                .iter()
                .filter(|path| reports(&wanted, path, false))
                .cloned()
                .collect()
        };

        assert_eq!(reported(&event(data, &[&file])), [file.as_path()]);
        // Other files in the same directory, such as a save's temporary file.
        assert!(reported(&event(data, &[&dir.join("other.txt")])).is_empty());
        assert!(reported(&event(data, &[&dir.join(".notes.txt.1.0.tmp")])).is_empty());
        // An atomic save renames a temporary file over the watched one.
        let temp = dir.join(".notes.txt.tmp");
        let both = EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        assert_eq!(reported(&event(both, &[&temp, &file])), [file.as_path()]);
        let to = EventKind::Modify(ModifyKind::Name(RenameMode::To));
        assert_eq!(reported(&event(to, &[&file])), [file.as_path()]);
        // Moving the file away or changing its permissions changes no text.
        let from = EventKind::Modify(ModifyKind::Name(RenameMode::From));
        assert!(reported(&event(from, &[&file])).is_empty());
        let chmod = EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions));
        assert!(reported(&event(chmod, &[&file])).is_empty());
        // `profiles/*.toml` covers every profile, even one created later.
        let created = EventKind::Create(CreateKind::File);
        let work = profiles.join("work.toml");
        assert_eq!(reported(&event(created, &[&work])), [work]);
        assert!(reported(&event(created, &[&profiles.join("work.toml.swp")])).is_empty());
        assert!(reported(&event(created, &[&dir.join("work.toml")])).is_empty());
    }

    #[test]
    fn ignores_roxannes_own_saves_only_where_asked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("main.rs");
        std::fs::write(&path, "fn main() {}\n").expect("write");
        let wanted = [normalize(&path)];
        assert!(reports(&wanted, &wanted[0], true));

        atomic_write(path.to_str().expect("path"), "fn main() {}\n// saved\n")
            .expect("atomic write");
        // The open file's watcher stays quiet, the config watcher reloads.
        assert!(!reports(&wanted, &wanted[0], true));
        assert!(reports(&wanted, &wanted[0], false));

        std::fs::write(&path, "fn main() { changed_by_another_program(); }\n").expect("write");
        assert!(reports(&wanted, &wanted[0], true));
    }
}
