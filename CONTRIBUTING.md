# Contribution à Roxanne

Merci de contribuer à Roxanne ! Ce guide précise le flux de travail attendu et les standards de qualité.

## Prérequis
- Rust stable installé via `rustup`.
- Connaissances de base de Git.

## Flux de contribution
1. Créer une branche dédiée pour votre changement.
2. Effectuer des commits clairs et ciblés.
3. Mettre à jour la documentation pertinente si nécessaire.
4. Exécuter les vérifications locales avant de proposer une PR.

## Vérifications locales
Exécuter, dans cet ordre si possible :

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

## Documentation
- Si vous modifiez une fonctionnalité, mettez à jour les fichiers correspondants dans `docs/`.
- Si la roadmap change, ajustez `tasks.md`.
- Si la planification évolue, mettez à jour `docs/roadmap.md`.

## Style de code
- Respecter `cargo fmt`.
- Éviter les `unwrap()` hors des tests.
- Écrire des tests dès que la logique est significative.

## Messages de commit
- Privilégier des messages courts à l’impératif (« Ajoute », « Corrige », « Documente »).
- Un commit doit correspondre à un sujet cohérent.

## Discussion et décisions
- Documenter les décisions techniques dans `docs/`.
- Référencer le contexte et l’impact pour les décisions importantes.
