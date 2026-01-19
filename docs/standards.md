# Standards de code – Roxanne

Ce document définit les conventions de code et les standards de qualité pour le
projet Roxanne.

## Langage et outilage
- **Langage principal** : Rust (édition stable).
- **Formatage** : `cargo fmt` (rustfmt, configuration par défaut).
- **Lint** : `cargo clippy --all-targets --all-features`.
- **CI** : exécuter `fmt`, `clippy` et les tests avant de proposer une
  contribution.

## Structure des modules
- Modules courts et focalisés (responsabilité unique).
- Éviter les dépendances cycliques entre modules.
- Préférer des interfaces explicites via `struct`/`trait` publics et des APIs
  minimales.
- Conserver un découplage clair entre **core**, **rendering**, **ui** et
  **plugins**.
- Éviter les singletons et l’état global partagé.

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
- Ajouter un contexte d’erreur (source, action en cours) pour le débogage.

## Tests
- Tests unitaires proches du code concerné (`mod tests`).
- Tests d’intégration dans `tests/` si nécessaire.
- Cibler les cas limites (fichiers volumineux, entrées invalides).
- Couvrir les invariants critiques (buffer, sélection, undo/redo).

## Performance
- Profiler avant d’optimiser.
- Éviter les allocations inutiles dans le rendu.
- Mesurer l’impact mémoire/CPU sur des scénarios réalistes.
- Documenter les résultats des benchmarks lorsque possible.

## Sécurité & robustesse
- Valider les entrées de fichiers (encodage, taille, erreurs d’I/O).
- Gérer les erreurs d’ouverture/sauvegarde sans perte de données.
- Prévoir des protections contre les crashs (sauvegarde de récupération).

## Commits
- Messages courts et descriptifs.
- Préférer un impératif (« Ajoute », « Corrige », « Documente »).
- Regrouper les changements logiquement liés.

## Revue de code
- Vérifier que chaque changement est accompagné d’une documentation pertinente.
- Mettre à jour `tasks.md` si la roadmap évolue.
- Signaler les impacts sur les performances ou l’architecture.
