<!-- roxanne-i18n: source=CONTRIBUTING.md sha=3ae214809c3a62e658229fc1ff299bbf47d03f3e -->

# Contribuer à Roxanne

Les corrections de bugs, les fonctionnalités et les améliorations de la
documentation sont les bienvenues. Cette page explique comment compiler
Roxanne, ce qu'il faut vérifier avant d'ouvrir une pull request, et à quoi
ressemble une bonne pull request.

## Prérequis

- Rust stable, installé avec [rustup](https://rustup.rs).
- La version minimale de Rust prise en charge (MSRV) est la `rust-version` de
  `Cargo.toml`, actuellement 1.90. La CI la vérifie : n'utilisez donc pas de
  fonctionnalités du langage ou de la bibliothèque standard plus récentes.

## Compiler et lancer

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo run
```

## Avant d'ouvrir une pull request

Lancez les mêmes vérifications que la CI :

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Si vous avez touché au buffer de texte ou au viewport, lancez aussi les
benchmarks :

```bash
cargo bench --bench text_buffer --bench viewport -- --noplot
python scripts/check_benchmarks.py
```

## Pull requests

- Un seul sujet par pull request. Nommez la branche d'après la tâche, par
  exemple `fix/save-as-overwrite` ou `feat/goto-symbol`.
- Le titre de la pull request est un [Conventional Commit](https://www.conventionalcommits.org)
  qui se lit comme l'entrée du changelog destinée aux utilisateurs :
  `feat: ...`, `fix: ...`, `perf: ...`, `refactor: ...`, `docs: ...`,
  `test: ...`, `ci: ...`, `build: ...` ou `chore: ...`.
- Chaque pull request est fusionnée par squash, si bien que son titre devient le
  commit sur `main`, et release-please construit le changelog et la version
  suivante à partir de ces commits.
- Si `main` a avancé, fusionnez-la dans votre branche avant de demander une
  revue.
- Quand une modification change un comportement, un chemin ou une commande,
  mettez à jour `docs/` dans la même pull request.

## Style de code

- Formatez avec `cargo fmt` (réglages rustfmt par défaut).
- Suivez les conventions de nommage de Rust : `snake_case` pour les fonctions,
  les variables et les modules, `CamelCase` pour les types et les traits,
  `SCREAMING_SNAKE_CASE` pour les constantes et les statics.
- Pas de `unwrap()` hors des tests : renvoyez une erreur ou affichez-la dans la
  barre d'état.
- Ajoutez des tests pour toute logique qui n'est pas triviale.
- Le code, les commentaires, la documentation, les messages de commit et les
  pull requests sont rédigés en anglais. Les traductions françaises de
  `docs/i18n/fr/` sont la seule exception. Quand vous modifiez une page anglaise
  qui en a une, mettez la traduction à jour dans la même pull request, puis
  lancez `scripts/i18n-check.sh --fix` pour la tamponner à nouveau : la CI
  échoue quand une traduction est plus ancienne que sa source anglaise.

## Structure des modules

- Gardez chaque module court et centré sur une seule tâche, et découpez celui
  qui commence à en faire plusieurs.
- Pas de dépendances cycliques entre modules. Le modèle de texte de `editor`
  ignore tout de la fenêtre, et rien en dehors de `ui` ne dépend de `ui` : la
  fenêtre utilise `editor`, `config`, `plugins` et le reste, jamais l'inverse.
  Gardez-le ainsi, pour que le cœur de l'éditeur, son rendu, l'interface et les
  extensions restent découplés.
- Les modules communiquent par des types et des fonctions publics, petits et
  explicites (`struct`, `enum`, `trait`) ; gardez tout le reste privé.
- Évitez les singletons et l'état global partagé : l'état vit dans la struct qui
  le détient, comme `RoxanneApp` ou un onglet, et il est transmis là où on en a
  besoin. En dehors des tests, les statics du code sont des tables en lecture
  seule construites une fois, comme les définitions de langages tree-sitter et
  les tables de traduction, et des compteurs qui gardent uniques les
  identifiants d'onglets et les noms de fichiers temporaires. La seule
  exception est `OWN_WRITES` dans `src/watch.rs`, qui note les fichiers que
  Roxanne a elle-même enregistrés pour que la surveillance des fichiers les
  ignore.

## Sécurité

Ne signalez pas de vulnérabilités dans des issues publiques. Voir
[SECURITY.md](../../../SECURITY.md) (en anglais).

## Licence

En contribuant, vous acceptez que vos contributions soient distribuées sous la
[GNU General Public License v3.0 ou ultérieure](../../../LICENSE)
(`GPL-3.0-or-later`).
