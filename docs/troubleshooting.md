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
