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
- Le script crée un `*.sha256` pour validation et agrège les hashes dans `dist/SHA256SUMS`.
- Pour signer l'archive (si `gpg` est disponible) :
  ```bash
  SIGN=1 scripts/package.sh --target x86_64-unknown-linux-gnu
  ```

### Périmètre des artefacts (Linux/macOS/Windows)
Chaque target génère un paquet `roxanne-<version>-<target>` avec le binaire,
la documentation et les licences.

| OS | Format | Nom attendu |
| --- | --- | --- |
| Linux | `.tar.gz` | `roxanne-<version>-<target>.tar.gz` |
| macOS | `.tar.gz` | `roxanne-<version>-<target>.tar.gz` |
| Windows | `.zip` | `roxanne-<version>-<target>.zip` |

Arborescence interne typique :
```
roxanne-<version>-<target>/
├─ roxanne[.exe]
├─ README.md
├─ LICENSE
└─ installation.md
```

Fichiers de checksum/signature attendus :
- `roxanne-<version>-<target>.<ext>.sha256`
- `SHA256SUMS` (agrège toutes les sommes)
- `roxanne-<version>-<target>.<ext>.asc` (si `SIGN=1` et `gpg` disponible)
- `roxanne-<version>-<target>.<ext>.asc.sha256` (si signature générée)

## Vérifier un artefact
```bash
sha256sum -c dist/roxanne-<version>-<target>.tar.gz.sha256
```
ou
```bash
sha256sum -c dist/SHA256SUMS
```

Pour une signature GPG :
```bash
gpg --verify dist/roxanne-<version>-<target>.<ext>.asc dist/roxanne-<version>-<target>.<ext>
```

## Désinstallation
- Supprimer l'exécutable et les fichiers de configuration locaux.
- Supprimer les archives `dist/` si besoin.
