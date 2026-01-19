# Architecture – Vue d’ensemble

## Modules principaux
- **Core** : gestion du buffer, opérations d’édition, undo/redo.
- **Rendering** : mise en page, rendu texte, cache de lignes.
- **UI** : entrées clavier/souris, commandes, composants.
- **Plugins** : API stable, sandboxing futur.

## Flux de données
1. Entrée utilisateur -> commandes UI.
2. Commandes -> mutations du core.
3. Core -> événements de rendu.
4. Rendering -> surface d’affichage.

## Conventions techniques
- Séparation stricte entre core et UI.
- Interfaces explicites entre modules.
- Événements et messages pour limiter le couplage.

## Décisions à figer
- Structure interne du buffer (rope vs gap vs piece table).
- Backend de rendu (CPU/GPU) et bibliothèque graphique.
