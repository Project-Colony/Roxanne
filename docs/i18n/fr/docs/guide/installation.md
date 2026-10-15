<!-- roxanne-i18n: source=docs/guide/installation.md sha=46f6d3c5bf82ba145815b021d89cf7a3e40c227e -->

# Installation

Roxanne est un seul exécutable, sans installateur. Choisissez l'une des trois
méthodes ci-dessous.

## Via Colony

Cherchez **Roxanne** dans [Colony](https://github.com/Project-Colony/Colony) et
installez-le. Colony vérifie les signatures de la version avant d'installer quoi
que ce soit, et apporte les mises à jour dès leur sortie.

## Binaires des versions publiées

Chaque version de la [page Releases](https://github.com/Project-Colony/Roxanne/releases)
fournit un binaire par plateforme :

| Plateforme | Fichier |
|---|---|
| Linux x86_64 | `roxanne-linux` |
| Windows x86_64 | `roxanne-windows.exe` |
| macOS, Apple Silicon | `roxanne-macos` |
| macOS, Intel | `roxanne-macos-x86` |

Chaque fichier est accompagné de `<asset>.sig`, `<asset>.meta` et
`<asset>.meta.sig`. La [Politique de signature du code](../../README.md#politique-de-signature-du-code)
du README explique comment les vérifier avec OpenSSL avant de lancer le binaire.

Sous Linux et macOS, rendez d'abord le fichier exécutable :

```bash
chmod +x roxanne-linux
./roxanne-linux
```

`roxanne --version` affiche la version et quitte sans ouvrir de fenêtre.

## Depuis les sources

Il vous faut Rust 1.90 ou plus récent, installé avec [rustup](https://rustup.rs).

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo build --release
./target/release/roxanne
```

## Désinstallation

Supprimez l'exécutable, ou désinstallez Roxanne depuis Colony. Vos réglages
restent dans `Colony/Roxanne/` sous votre dossier de configuration
(`~/.config/Colony/Roxanne/` sous Linux, `%LOCALAPPDATA%\Colony\Roxanne\` sous
Windows, `~/Library/Application Support/Colony/Roxanne/` sous macOS) :
supprimez aussi ce dossier pour les effacer. Des réglages antérieurs au passage
à ce dossier peuvent encore se trouver dans `~/.config/roxanne/`, que Roxanne ne
supprime jamais.
