# Tests – Stratégie

## Types de tests
- Unitaires pour le core.
- Intégration pour les flux UI -> core.
- Tests de performance (benchmarks).

## Données de test
- Fichiers petits, moyens, volumineux.
- Cas de stress (sélections multiples, gros collage).

## Outils
- `cargo test`.
- Benchmarks dédiés (à définir).

## Planification des tests
- Phase 1 : tests unitaires sur buffer, sélection, I/O.
- Phase 2 : tests d'intégration sur recherche/undo/redo.
- Phase 4 : benchmarks automatisés.
