# Build & Release – Processus

## Build local
- `cargo build` pour build standard.
- `cargo build --release` pour build optimisé.

## Distribution
- Packaging via `scripts/package.sh` (archives + checksum).
- Exemple (Linux) :
  ```bash
  scripts/package.sh --target x86_64-unknown-linux-gnu
  ```
- Signature optionnelle :
  ```bash
  SIGN=1 scripts/package.sh --target x86_64-unknown-linux-gnu
  ```
- Vérification d'archive :
  ```bash
  sha256sum -c dist/roxanne-<version>-<target>.tar.gz.sha256
  ```

## Versioning
- SemVer recommandé.
- Changelog à maintenir.

## CI/CD (future)
- Tests automatiques.
- Lint et format.
- Publication des artefacts.

## Planification release
- Phase 4 : pipeline de build multi-plateforme + packaging.
- Phase 4 : documentation d'installation et notes de version.
