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
  - **Décision** : adopter une **rope** pour optimiser les insertions/suppressions sur gros fichiers.
  - **Raison** : bonnes performances sur les éditions au milieu du document, coût mémoire acceptable.
  - **Impact** : nécessité d’implémenter un mapping précis entre positions logiques et index.
- Backend de rendu (CPU/GPU) et bibliothèque graphique.
  - **Décision** : rendu **CPU** initial avec une bibliothèque multiplateforme (à préciser).
  - **Raison** : prioriser la simplicité et la stabilité pour le MVP.
  - **Impact** : prévoir une abstraction pour migrer vers un backend GPU plus tard.
