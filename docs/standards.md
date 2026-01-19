# Standards de code – Roxanne

Ce document définit les conventions de code et les standards de qualité pour le projet Roxanne.

## Langage et outilage
- **Langage principal** : Rust (édition stable).
- **Formatage** : `cargo fmt` (rustfmt, configuration par défaut).
- **Lint** : `cargo clippy --all-targets --all-features`.
- **CI** : exécuter `fmt`, `clippy` et les tests avant de proposer une contribution.

## Structure des modules
- Modules courts et focalisés (responsabilité unique).
- Éviter les dépendances cycliques entre modules.
- Préférer des interfaces explicites via `struct`/`trait` publics et des APIs minimales.
- Conserver un découplage clair entre **core**, **rendering**, **ui** et **plugins**.

## Conventions de nommage
- `snake_case` pour fonctions, variables, modules.
- `CamelCase` pour types et traits.
- Constantes en `SCREAMING_SNAKE_CASE`.

## Documentation du code
- Documenter les APIs publiques avec `///`.
- Ajouter des exemples si une API est non triviale.
- Préférer des commentaires expliquant *le pourquoi* plutôt que *le quoi*.

## Gestion des erreurs
- Utiliser `Result` et des erreurs explicites.
- Éviter `unwrap()` et `expect()` en production, sauf dans les tests.
- Centraliser les erreurs dans un module dédié si nécessaire.

## Tests
- Tests unitaires proches du code concerné (`mod tests`).
- Tests d’intégration dans `tests/` si nécessaire.
- Cibler les cas limites (fichiers volumineux, entrées invalides).

## Performance
- Profiler avant d’optimiser.
- Éviter les allocations inutiles dans le rendu.
- Mesurer l’impact mémoire/CPU sur des scénarios réalistes.

## Commits
- Messages courts et descriptifs.
- Préférer un impératif (« Ajoute », « Corrige », « Documente »).
- Regrouper les changements logiquement liés.
