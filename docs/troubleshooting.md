# Troubleshooting – Roxanne

## Build échoue avec Rust/Cargo
- Vérifier la version de Rust :
  ```bash
  rustc --version
  ```
- Mettre à jour via `rustup` :
  ```bash
  rustup update stable
  ```

## L'exécutable ne se lance pas
- Vérifier que le binaire existe :
  ```bash
  ls -l target/release/roxanne
  ```
- Relancer un build propre :
  ```bash
  cargo clean
  cargo build --release
  ```

## Packaging échoue
- Vérifier que la cible est installée :
  ```bash
  rustup target list --installed
  ```
- Ajouter une cible si nécessaire :
  ```bash
  rustup target add x86_64-unknown-linux-gnu
  ```
- Rejouer le script :
  ```bash
  scripts/package.sh --target x86_64-unknown-linux-gnu
  ```

## Problème de signature GPG
- `SIGN=1` nécessite `gpg` dans le PATH.
- Vérifier la configuration GPG :
  ```bash
  gpg --list-keys
  ```
