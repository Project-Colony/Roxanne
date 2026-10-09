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

## Release downloads
Each release on the [Releases page](https://github.com/Project-Colony/Roxanne/releases)
ships one binary per platform, which Colony picks and installs for you:

| Platform | Asset |
| --- | --- |
| Linux x86_64 | `roxanne-linux` |
| Windows x86_64 | `roxanne-windows.exe` |
| macOS, Apple Silicon | `roxanne-macos` |
| macOS, Intel | `roxanne-macos-x86` |

Every asset comes with `<asset>.sig`, an ed25519 signature by the Project Colony
release key, and `<asset>.meta` with its own signature `<asset>.meta.sig`, which
names the version, the asset and its SHA-256. To check a download with OpenSSL 3,
save the public key as `colony-release.pub`:

```
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
```

then run:

```bash
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in roxanne-linux -sigfile roxanne-linux.sig
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in roxanne-linux.meta -sigfile roxanne-linux.meta.sig
cat roxanne-linux.meta
sha256sum roxanne-linux
```

Both `openssl` commands must print `Signature Verified Successfully`, and
`roxanne-linux.meta` must read exactly `version=<tag>`, `asset=roxanne-linux` and
`sha256=` followed by the digest that `sha256sum` prints.

## Désinstallation
- Supprimer l'exécutable et les fichiers de configuration locaux.
