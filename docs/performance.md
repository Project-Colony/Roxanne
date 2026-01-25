# Performance – Objectifs et mesures

## Objectifs
- Démarrage < 200 ms sur machine modeste.
- Navigation fluide sur fichiers 1-5 MB.
- RAM stable au repos.

## Mesures
- Temps de démarrage.
- FPS en défilement.
- Latence des commandes critiques.

## Stratégies
- Cache de rendu.
- Allocation mémoire maîtrisée.
- Profilage régulier.

## Planification
- Phase 1 : mesures de démarrage et de rendu sur un projet moyen.
- Phase 2 : seuils de latence pour la recherche et le highlight.
- Phase 4 : benchmarks automatisés en CI (scénarios TextBuffer prêts).
