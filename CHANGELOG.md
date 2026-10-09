# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added
- Governance documents (LICENSE, Code of Conduct, SECURITY).
- GitHub templates for issues and pull requests.
- Initial changelog.

### Fixed
- Saving keeps the file's permissions (a `0600` file stays `0600`) and, on
  Unix, its owner and group where the user may set them. Saving through a
  symlink updates its target and keeps the link.
- A failed save leaves the original file untouched. It used to delete the only
  copy when restoring the backup failed.
- Saving a new tab asks for a file name instead of writing `untitled.txt` in
  the working directory, and Save As asks before replacing an existing file.
- Reloading a file changed on disk reloads that file's tab, not the active one,
  and never replaces unsaved edits or their undo history. Opening a search
  result in a file with unsaved edits keeps them as well.
- An untouched file without a final newline no longer shows as modified.
- Config hot reload no longer starts a thread every second. It now covers
  profiles and config files created after startup, and keeps working after an
  editor saves by renaming a new file over the old one. Removing or moving away
  a config file or profile reloads the config as well, and a config directory
  that is removed and restored keeps being watched.
- Files opened after startup are watched for changes made by other programs,
  and saving a file in Roxanne no longer asks whether to reload it.
- Opening a file from the file tree no longer renames the current tab.

### Security
- Native plugins (`plugins.dynamic`) are only loaded from the user's own
  configuration and the profiles it selects. A `.roxanne.toml` in the opened
  folder or a parent, and any profile it selects, can no longer list them; its
  `plugins.dynamic` is ignored with a warning.
- Native plugin paths must be absolute. A relative path is refused, because it
  would resolve against the working directory.
- On Windows, a native plugin's DLL dependencies are looked up in the plugin's
  own folder, Roxanne's folder and System32, never in the working directory.
