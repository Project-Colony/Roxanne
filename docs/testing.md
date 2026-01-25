# Tests – Stratégie

## Types de tests
- Unitaires pour le core.
- Intégration pour les flux UI -> core (config multi-niveaux, I/O atomique).
- Tests de performance (benchmarks).

## Données de test
- Fichiers petits, moyens, volumineux.
- Cas de stress (sélections multiples, gros collage).

## Outils
- `cargo test`.
- Benchmarks dédiés avec Criterion (`cargo bench`).

## Planification des tests
- Phase 1 : tests unitaires sur buffer, sélection, I/O.
- Phase 2 : tests d'intégration sur recherche/undo/redo.
- Phase 4 : tests d'intégration initiaux (config multi-niveaux, I/O atomique) + benchmarks automatisés.
