# Changelog

## [0.1.0](https://github.com/Project-Colony/Roxanne/compare/v0.1.0...v0.1.0) (2026-10-09)


### Fixes

* **deps:** update bytes, rand and xxhash-rust for security advisories ([#2](https://github.com/Project-Colony/Roxanne/issues/2)) ([9c9c19f](https://github.com/Project-Colony/Roxanne/commit/9c9c19f8b601066393ac6b444cb84f2ab0e927fe))
* **files:** keep permissions and symlinks when saving and never lose unsaved edits ([#4](https://github.com/Project-Colony/Roxanne/issues/4)) ([438c6aa](https://github.com/Project-Colony/Roxanne/commit/438c6aaf1effdc61de6b36b002c6ed992762ce65))
* **lsp:** stop freezing the editor and never run a project's build scripts ([#7](https://github.com/Project-Colony/Roxanne/issues/7)) ([0822674](https://github.com/Project-Colony/Roxanne/commit/08226743f9327626be6b8ddda5c998775f5f7a6c))
* **plugins:** never load native plugins from a project config ([#3](https://github.com/Project-Colony/Roxanne/issues/3)) ([204b314](https://github.com/Project-Colony/Roxanne/commit/204b314176be717bdb2eb08c94f4da59fa671330))
* **watch:** reload the config when a config file is removed and keep watching a restored config directory ([#6](https://github.com/Project-Colony/Roxanne/issues/6)) ([2ad6202](https://github.com/Project-Colony/Roxanne/commit/2ad6202be8058c0438666c309f941e7199b831f9))
* **watch:** watch every open file and config reliably without spawning a thread per poll ([#5](https://github.com/Project-Colony/Roxanne/issues/5)) ([4b7fd8f](https://github.com/Project-Colony/Roxanne/commit/4b7fd8fdee281bb6684ba9bf0ddceaaaa1d63b39))


### CI

* build, sign and publish releases through release-please and the shared workflow ([#8](https://github.com/Project-Colony/Roxanne/issues/8)) ([6dbf818](https://github.com/Project-Colony/Roxanne/commit/6dbf818da9cc0f9455e4c4a1f9a7e0b8568717d3))
