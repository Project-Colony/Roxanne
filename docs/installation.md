# Installation – Roxanne

Ce document décrit les options d'installation et d'exécution de Roxanne.

## Prérequis
- **Rust** via `rustup` (stable recommandé).
- **Cargo** (fourni avec Rust).

## Installer depuis les sources
1. Cloner le dépôt.
2. Construire l'application :
   ```bash
   cargo build --release
   ```
3. Lancer l'exécutable :
   ```bash
   ./target/release/roxanne
   ```

## Packaging local (artefacts distribuables)
Pour générer un paquet avec checksum et signature optionnelle :
```bash
scripts/package.sh --target x86_64-unknown-linux-gnu
```
- Les artefacts sont placés dans `dist/`.
- Le script crée un `*.sha256` pour validation.
- Pour signer l'archive (si `gpg` est disponible) :
  ```bash
  SIGN=1 scripts/package.sh --target x86_64-unknown-linux-gnu
  ```

## Vérifier un artefact
```bash
sha256sum -c dist/roxanne-<version>-<target>.tar.gz.sha256
```

## Désinstallation
- Supprimer l'exécutable et les fichiers de configuration locaux.
- Supprimer les archives `dist/` si besoin.
