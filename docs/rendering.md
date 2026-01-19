# Rendering – Pipeline de rendu

## Objectifs
- Rendu fluide à 60+ FPS sur des fichiers moyens.
- Défilement sans saccade.
- Consommation CPU maîtrisée.

## Pipeline préliminaire
1. Mesure et mise en forme des lignes visibles.
2. Cache des glyphes et lignes.
3. Rendu incrémental sur changements locaux.

## Optimisations
- Cache de lignes visibles.
- Réutilisation des buffers.
- Limitation des recalculs de layout.

## Décisions à valider
- Backend (CPU/GPU) et librairie graphique.
- Gestion de la mise en forme et du shaping.
