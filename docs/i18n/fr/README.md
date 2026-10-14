<!-- roxanne-i18n: source=README.md sha=1df07e140380ddc71c686e128e5b25e4ea169e3a -->

<div align="center">

# Roxanne

**Un éditeur de code léger pour les développeurs qui veulent un éditeur rapide, pensé pour le clavier, sans IDE autour.**

</div>

[![Licence : GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Application Colony](https://img.shields.io/badge/Colony-development-purple)](https://github.com/Project-Colony/Colony)
[![Plateformes](https://img.shields.io/badge/platforms-linux%20%7C%20windows%20%7C%20macOS-lightgrey)](#installation)

Roxanne est un éditeur de code natif pour le bureau, écrit en Rust avec
[iced](https://iced.rs). Un IDE exige un projet avant d'ouvrir un fichier et
exécute le code de ce projet pour le comprendre, tandis qu'un simple éditeur de
texte ne sait rien du code. Roxanne ouvre n'importe quel fichier sans projet,
colore huit langages avec tree-sitter, et n'interroge rust-analyzer sur du code
Rust que lorsque vous utilisez une action LSP.

> **État :** développement précoce, pas encore prêt pour un usage quotidien. La
> CI compile et teste Roxanne sous Linux à chaque modification et le vérifie avec
> Rust 1.90 ; le workflow de publication produit les binaires Linux, Windows et
> macOS, qui n'ont pas encore servi au quotidien. La prise en charge des serveurs
> de langage est expérimentale et ne couvre que Rust, via rust-analyzer : survol
> et aller à la définition.

## Pourquoi Roxanne

- **Aucun projet nécessaire.** Saisissez un chemin et ouvrez le fichier. Un
  `.roxanne.toml` dans un dossier ajoute des réglages de projet, mais rien n'en
  exige un.
- **Ouvrir un dossier n'exécute rien.** Les réglages d'un projet ne peuvent
  jamais charger une extension ni lancer un programme. rust-analyzer ne démarre
  que lorsque vous demandez une action LSP, avec les scripts de build et les
  macros procédurales désactivés.
- **Des enregistrements fiables.** Chaque enregistrement remplace le fichier en
  un seul renommage, conserve ses permissions et ses liens symboliques, et un
  rechargement ne jette jamais des modifications non enregistrées.
- **Le clavier d'abord.** Des menus, une palette de commandes et des raccourcis
  que vous pouvez redéfinir, avec des profils de raccourcis et un mode normal
  facultatif inspiré de vim.

## Ce qu'il fait

- Onglets, annuler et rétablir, et curseurs multiples.
- Coloration syntaxique avec tree-sitter pour Rust, C, Go, JavaScript, JSON,
  Markdown, Python et TOML.
- Rechercher et remplacer dans le fichier ouvert, avec expressions régulières et
  groupes de capture, et recherche dans les fichiers de tout l'espace de travail.
- Une arborescence de fichiers, une palette de commandes, aller à la ligne, une
  minicarte, le retour à la ligne et un panneau de diagnostics pour les
  délimiteurs non appariés et les chaînes non terminées.
- Prise en charge expérimentale du Language Server Protocol pour Rust : survol
  et aller à la définition via `rust-analyzer` trouvé dans votre `PATH`, démarré
  pour l'espace de travail Cargo du fichier concerné et qui répond en
  arrière-plan. Il tourne avec les scripts de build, les macros procédurales et
  `cargo check` désactivés, mais les propres fichiers `rust-analyzer.toml`,
  configuration Cargo et `rust-toolchain.toml` d'un projet changent encore ce
  qu'il exécute : n'utilisez les actions LSP que sur du code de confiance. Voir
  [Serveur de langage](docs/guide/configuration.md#lsp).
- Les Préférences, ouvertes d'un clic sur **Roxanne** en haut à gauche : les
  thèmes et accents Colony, la taille du texte, le contraste élevé, la police
  OpenDyslexic, la réduction des animations, l'anglais ou le français, et les
  options propres à l'éditeur. Une modification s'applique aussitôt et est
  enregistrée dans votre `config.toml`, ou dans le profil qui la définit.
- Une configuration TOML avec profils de raccourcis et rechargement à chaud, et
  des extensions intégrées et natives pour la barre d'état.
- Un enregistrement sûr : le texte va dans un fichier temporaire à côté du vrai,
  qui le remplace en un seul renommage. Quand un autre programme modifie un
  fichier ouvert, Roxanne demande avant de le recharger.

## Installation

### Via Colony (recommandé)

Cherchez **Roxanne** dans [Colony](https://github.com/Project-Colony/Colony) et
installez-le. Les mises à jour arrivent par le lanceur.

### Téléchargement direct du binaire

Prenez le fichier de votre plateforme dans la
[dernière version](https://github.com/Project-Colony/Roxanne/releases/latest) :

| Plateforme | Fichier |
|---|---|
| Linux | `roxanne-linux` |
| Windows | `roxanne-windows.exe` |
| macOS (Apple Silicon) | `roxanne-macos` |
| macOS (Intel) | `roxanne-macos-x86` |

```bash
chmod +x roxanne-linux && ./roxanne-linux
```

`roxanne --version` affiche la version installée.

### Compiler depuis les sources

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo build --release
./target/release/roxanne
```

Nécessite Rust 1.90 ou plus récent, installé avec [rustup](https://rustup.rs).

## Documentation

La documentation complète se trouve dans [docs/](docs/). Commencez par
[docs/README.md](docs/README.md).

## Langue

La documentation existe aussi en français. **L'anglais fait foi** : quand une
traduction le contredit, c'est le fichier anglais qui a raison.

- **Français** - [README](README.md) · [index](docs/README.md) · [installation](docs/guide/installation.md) · [utilisation](docs/guide/usage.md) · [configuration](docs/guide/configuration.md) · [extensions](docs/guide/plugins.md) · [dépannage](docs/guide/troubleshooting.md) · [présentation](docs/project/overview.md) · [architecture](docs/internals/architecture.md) · [cœur de l'éditeur](docs/internals/editor-core.md) · [rendu](docs/internals/rendering.md) · [performance](docs/internals/performance.md) · [tests](docs/internals/testing.md) · [contribuer](CONTRIBUTING.md) · [code de conduite](.github/CODE_OF_CONDUCT.md)

Chaque traduction porte l'empreinte du fichier anglais dont elle est issue, et la
CI échoue quand l'anglais prend de l'avance ; voir [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
`docs/i18n/fr/` reproduit la racine du dépôt : `docs/i18n/fr/docs/guide/usage.md`
est la version française de `docs/guide/usage.md`, et un lien entre deux pages
traduites est le même qu'entre leurs originaux anglais.

## Politique de signature du code

Chaque fichier publié est signé par l'organisation Project Colony dans la CI,
jamais sur la machine d'un développeur. À côté de chaque fichier, sur la page de
la version :

| Fichier | Ce que c'est |
|---|---|
| `<asset>.sig` | une signature ed25519 du fichier, faite avec la clé de publication de l'organisation |
| `<asset>.meta` | trois lignes qui lient le fichier à son nom, à son sha256 et à la version publiée |
| `<asset>.meta.sig` | une signature ed25519 du `.meta` |

La clé privée est un secret de l'organisation, utilisé uniquement par le
[workflow partagé de signature et de publication](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
dans un job qui ne compile rien ; les jobs qui compilent Roxanne ne la voient
jamais. Colony vérifie les trois fichiers avant d'installer ou de mettre à jour
Roxanne, et refuse une version plus ancienne que celle installée. Pour vérifier
vous-même un téléchargement avec OpenSSL 3 (les mêmes commandes valent pour
chaque fichier) :

```bash
cat > colony-release.pub <<'KEY'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
KEY
a=roxanne-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<nom du fichier>, sha256=<empreinte>
sha256sum "$a"    # l'empreinte doit être égale à la ligne sha256
```

Comment les versions sont compilées, signées et publiées :
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)
(en anglais).

## Confidentialité

Roxanne n'envoie ni télémétrie, ni statistiques d'usage, ni rapports de plantage,
et n'ouvre aucune connexion réseau de lui-même.

| Données | Stockées ou envoyées | Où, et pourquoi |
|---|---|---|
| Réglages | stockés | `config.toml` et les profils de `profiles/`, dans `Colony/Roxanne/` sous votre dossier de configuration (`~/.config/Colony/Roxanne/` sous Linux). Roxanne les lit, et la page Préférences écrit chaque réglage que vous modifiez dans `config.toml`, ou dans le profil qui le définit. |
| Anciens réglages | lus, copiés | `config.toml` et `profiles/` depuis `~/.config/roxanne/`, copiés une seule fois dans `Colony/Roxanne/` quand celui-ci n'a pas encore de `config.toml`. Un fichier `.migrated-from-legacy-config` y marque la copie comme faite. L'ancien dossier n'est jamais modifié. |
| Réglages de projet | lus | le premier `.roxanne.toml` trouvé dans le dossier de travail ou l'un de ses parents |
| Vos fichiers | stockés | là où vous les enregistrez. Un enregistrement écrit un fichier temporaire à côté du fichier puis le renomme par-dessus, ce qui conserve ses permissions, et un lien symbolique reste un lien. |
| Texte du fichier ouvert | envoyé à rust-analyzer | seulement quand vous utilisez une action LSP sur un fichier Rust enregistré. rust-analyzer est lancé depuis votre `PATH`, ou comme le programme défini dans les Préférences (`[lsp]` dans votre propre `config.toml`), et tourne sur cette machine avec les scripts de build et les macros procédurales désactivés. Il exécute `cargo metadata`, si bien que Cargo ou rustup peuvent télécharger les dépendances ou la chaîne d'outils du projet. |
| Texte et nom du fichier ouvert | envoyés aux extensions natives | seulement les extensions listées dans votre propre `config.toml` ou dans un profil qu'il sélectionne. Elles tournent avec vos droits, et Roxanne ne limite pas ce qu'elles en font. |

Ce programme ne transmet aucune information à d'autres systèmes en réseau, sauf
à la demande expresse de l'utilisateur ou de la personne qui l'installe ou
l'exploite.

## Licence

Roxanne est distribué sous la [GNU General Public License v3.0 ou ultérieure](../../../LICENSE).

Roxanne embarque les polices Colony via [colony-ui](https://crates.io/crates/colony-ui) :
JetBrainsMono Nerd Font, OpenDyslexic et Font Awesome 6 Free, chacune sous la
SIL Open Font License 1.1.
