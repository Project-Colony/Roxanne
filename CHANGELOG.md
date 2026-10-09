# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added
- Governance documents (LICENSE, Code of Conduct, SECURITY).
- GitHub templates for issues and pull requests.
- Initial changelog.

### Security
- Native plugins (`plugins.dynamic`) are only loaded from the user's own
  configuration and the profiles it selects. A `.roxanne.toml` in the opened
  folder or a parent, and any profile it selects, can no longer list them; its
  `plugins.dynamic` is ignored with a warning.
- Native plugin paths must be absolute. A relative path is refused, because it
  would resolve against the working directory.
- On Windows, a native plugin's DLL dependencies are looked up in the plugin's
  own folder, Roxanne's folder and System32, never in the working directory.
