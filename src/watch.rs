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
use notify::event::ModifyKind;
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
/// the paths do. A directory that does not exist yet, or that is removed, is
/// watched from its parent until it is created. With `ignore_own_writes`, for
/// open files, a file that is still the one Roxanne last saved, or that is no
/// longer there, is not reported: the open text stays. Without it, a removed
/// file is reported too, so a config reload sees it gone.
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

        let files = paths().iter().map(|path| normalize(path)).collect();
        let mut watched = Watched::new(&mut watcher, files, ignore_own_writes);
        while let Some(event) = events.next().await {
            // One save often raises several events: take all that are queued.
            let mut batch = vec![event];
            while let Ok(Some(event)) = events.try_next() {
                batch.push(event);
            }
            for path in watched.changed(&mut watcher, batch) {
                let _ = output.send(on_change(path)).await;
            }
        }
        // The watcher holds the sender, so the events never end.
        loop {
            std::future::pending::<()>().await;
        }
    })
}

/// The files a watcher reports, and the directories it watches for them.
struct Watched {
    files: Vec<PathBuf>,
    ignore_own_writes: bool,
    /// The directories being watched: those of the files, and the parents of
    /// the missing ones.
    watching: Vec<PathBuf>,
    /// The directories of files that do not exist: their parent is watched, so
    /// their creation is seen.
    missing: Vec<PathBuf>,
}

impl Watched {
    fn new(watcher: &mut impl Watcher, files: Vec<PathBuf>, ignore_own_writes: bool) -> Self {
        // Every directory starts out missing, until its watch begins.
        let missing = files
            .iter()
            .filter_map(|file| Some(file.parent()?.to_path_buf()))
            .collect();
        let mut watched = Self {
            files,
            ignore_own_writes,
            watching: Vec::new(),
            missing,
        };
        watched.watch_missing(watcher);
        watched
    }

    /// Follows a batch of events and returns the files to report, each once.
    fn changed(
        &mut self,
        watcher: &mut impl Watcher,
        events: impl IntoIterator<Item = notify::Result<Event>>,
    ) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        let mut recheck = false;
        for event in events.into_iter().flatten() {
            if matches!(
                event.kind,
                EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
            ) {
                // A watched directory was removed, which ends its watch, or
                // moved away, which takes its watch along.
                for path in &event.paths {
                    if let Some(index) = self.watching.iter().position(|dir| dir == path) {
                        let _ = watcher.unwatch(path);
                        self.watching.swap_remove(index);
                        changed.extend(self.files_in(path));
                        self.missing.push(path.clone());
                        recheck = true;
                    }
                }
            }
            for path in changed_paths(&event) {
                if self.missing.contains(path) {
                    recheck = true;
                } else {
                    changed.push(path.clone());
                }
            }
        }
        if recheck {
            // Files in a directory found again may have been written before
            // its watch started.
            changed.extend(self.watch_missing(watcher));
        }
        changed.retain(|path| reports(&self.files, path, self.ignore_own_writes));
        changed.sort();
        changed.dedup();
        changed
    }

    /// Watches each missing directory that exists now, and the parent of each
    /// other one. Returns the files in the directories it found.
    fn watch_missing(&mut self, watcher: &mut impl Watcher) -> Vec<PathBuf> {
        let mut found = Vec::new();
        // Parents first, so a directory created with another one below it
        // finds both.
        self.missing.sort();
        self.missing.dedup();
        for dir in std::mem::take(&mut self.missing) {
            if self.watch(watcher, &dir) {
                found.extend(self.files_in(&dir));
            } else {
                if let Some(parent) = dir.parent() {
                    self.watch(watcher, parent);
                }
                self.missing.push(dir);
            }
        }
        found
    }

    /// Watches `dir` unless it already is, and returns whether it is watched.
    fn watch(&mut self, watcher: &mut impl Watcher, dir: &Path) -> bool {
        if self.watching.iter().any(|watched| watched == dir) {
            return true;
        }
        let watched = watcher.watch(dir, RecursiveMode::NonRecursive).is_ok();
        if watched {
            self.watching.push(dir.to_path_buf());
        }
        watched
    }

    fn files_in(&self, dir: &Path) -> Vec<PathBuf> {
        let in_dir = |file: &&PathBuf| file.parent() == Some(dir);
        self.files.iter().filter(in_dir).cloned().collect()
    }
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

/// The paths an event created, changed or removed. A rename counts for both
/// names, and a metadata change (permissions, times) not at all.
fn changed_paths(event: &Event) -> &[PathBuf] {
    match event.kind {
        EventKind::Modify(ModifyKind::Metadata(_)) => &[],
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => &event.paths,
        _ => &[],
    }
}

/// Whether a change to `path` is reported: it is one of the `wanted` paths
/// (where `dir/*.toml` stands for every TOML file in `dir`), and with
/// `ignore_own_writes`, the file is there and is not the one Roxanne last
/// saved.
fn reports(wanted: &[PathBuf], path: &Path, ignore_own_writes: bool) -> bool {
    let is_wanted = wanted.iter().any(|wanted| {
        wanted == path
            || (wanted.file_name() == Some(OsStr::new("*.toml"))
                && wanted.parent() == path.parent()
                && path.extension() == Some(OsStr::new("toml")))
    });
    is_wanted && !(ignore_own_writes && (!path.exists() || is_own_write(path)))
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
    use notify::event::{CreateKind, DataChange, MetadataKind, RemoveKind, RenameMode};
    use std::sync::mpsc::Receiver;
    use std::time::Duration;

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

        let reported = |event: &Event, ignore_own_writes: bool| -> Vec<PathBuf> {
            changed_paths(event)
                .iter()
                .filter(|path| reports(&wanted, path, ignore_own_writes))
                .cloned()
                .collect()
        };

        std::fs::write(&file, "notes\n").expect("write");
        assert_eq!(reported(&event(data, &[&file]), true), [file.as_path()]);
        // Other files in the same directory, such as a save's temporary file.
        assert!(reported(&event(data, &[&dir.join("other.txt")]), false).is_empty());
        assert!(reported(&event(data, &[&dir.join(".notes.txt.1.0.tmp")]), false).is_empty());
        // An atomic save renames a temporary file over the watched one.
        let temp = dir.join(".notes.txt.tmp");
        let both = EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        assert_eq!(
            reported(&event(both, &[&temp, &file]), true),
            [file.as_path()]
        );
        let to = EventKind::Modify(ModifyKind::Name(RenameMode::To));
        assert_eq!(reported(&event(to, &[&file]), true), [file.as_path()]);
        // Changing permissions changes no text.
        let chmod = EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions));
        assert!(reported(&event(chmod, &[&file]), false).is_empty());
        // A file removed or moved away: the config reloads, an open file keeps
        // its text.
        std::fs::remove_file(&file).expect("remove");
        let from = EventKind::Modify(ModifyKind::Name(RenameMode::From));
        let removed = EventKind::Remove(RemoveKind::File);
        let backup = dir.join("notes.txt.bak");
        for event in [
            event(from, &[&file]),
            event(removed, &[&file]),
            event(both, &[&file, &backup]),
        ] {
            assert_eq!(reported(&event, false), [file.as_path()]);
            assert!(reported(&event, true).is_empty());
        }
        // `profiles/*.toml` covers every profile, even one created later.
        let created = EventKind::Create(CreateKind::File);
        let work = profiles.join("work.toml");
        assert_eq!(reported(&event(created, &[&work]), false), [work]);
        assert!(reported(&event(created, &[&profiles.join("work.toml.swp")]), false).is_empty());
        assert!(reported(&event(created, &[&dir.join("work.toml")]), false).is_empty());
    }

    /// Feeds events to `watched` until `done` holds for it and the files it
    /// reported.
    fn follow(
        watched: &mut Watched,
        watcher: &mut impl Watcher,
        events: &Receiver<notify::Result<Event>>,
        done: impl Fn(&Watched, &[PathBuf]) -> bool,
    ) {
        let mut reported = Vec::new();
        while !done(watched, &reported) {
            let event = events
                .recv_timeout(Duration::from_secs(10))
                .expect("an event within 10 seconds");
            reported
                .extend(watched.changed(watcher, std::iter::once(event).chain(events.try_iter())));
        }
    }

    #[test]
    fn keeps_watching_a_directory_that_is_removed_moved_or_created() {
        let root = tempfile::tempdir().expect("tempdir");
        let root = normalize(root.path());
        let dir = root.join("roxanne");
        std::fs::create_dir(&dir).expect("create dir");
        let (sender, events) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(sender).expect("watcher");
        let mut watched = Watched::new(&mut watcher, vec![dir.join("*.toml")], false);
        assert!(watched.missing.is_empty());

        let wrote = |name: &str| {
            let file = dir.join(name);
            std::fs::write(&file, "a = 1\n").expect("write");
            move |_: &Watched, reported: &[PathBuf]| reported.contains(&file)
        };
        let missing = |watched: &Watched, _: &[PathBuf]| watched.missing == [dir.clone()];
        let found = |watched: &Watched, _: &[PathBuf]| watched.missing.is_empty();

        follow(&mut watched, &mut watcher, &events, wrote("a.toml"));
        // Removing the directory ends its watch: its parent sees it come back.
        std::fs::remove_dir_all(&dir).expect("remove dir");
        follow(&mut watched, &mut watcher, &events, missing);
        std::fs::create_dir(&dir).expect("create dir");
        follow(&mut watched, &mut watcher, &events, found);
        follow(&mut watched, &mut watcher, &events, wrote("b.toml"));
        // A watch follows a directory moved away, so it must be replaced.
        std::fs::rename(&dir, root.join("old")).expect("rename dir");
        follow(&mut watched, &mut watcher, &events, missing);
        std::fs::create_dir(&dir).expect("create dir");
        follow(&mut watched, &mut watcher, &events, found);
        follow(&mut watched, &mut watcher, &events, wrote("c.toml"));
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
