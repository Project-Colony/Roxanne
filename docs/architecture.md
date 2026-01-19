# Architecture – Vue d’ensemble

## Modules principaux
- **Core** : gestion du buffer, opérations d’édition, undo/redo, sélection.
- **Rendering** : mise en page, rendu texte, cache de lignes, viewport.
- **UI** : entrées clavier/souris, commandes, composants, layout.
- **Plugins** : API stable, events/hook, sandboxing futur.
- **Persistence** : chargement/sauvegarde des fichiers, état de session.

## Flux de données
1. Entrée utilisateur -> commandes UI.
2. Commandes -> mutations du core.
3. Core -> événements de rendu + recalcul des régions impactées.
4. Rendering -> surface d’affichage.
5. Persistence -> sérialisation ou rechargement à la demande.

## Principes d’architecture
- Séparation stricte entre core et UI.
- Interfaces explicites entre modules (traits, messages, événements).
- Éviter l’état global : privilégier des structures de données transmises.
- Favoriser les structures immuables pour les snapshots d’historique.

## Décisions structurantes
### Structure du buffer
- **Décision** : adopter une **rope** pour optimiser les insertions/suppressions
  sur gros fichiers.
- **Raison** : bonnes performances sur les éditions au milieu du document, coût
  mémoire acceptable.
- **Impact** : implémenter un mapping précis entre positions logiques et index.
- **Alternatives évaluées** : gap buffer (simple mais dégradé sur gros fichiers),
  piece table (plus complexe, coût mémoire variable).

### Backend de rendu
- **Décision** : rendu **CPU** initial avec une bibliothèque multiplateforme
  (à préciser à la Phase 1).
- **Raison** : prioriser la simplicité et la stabilité pour le MVP.
- **Impact** : prévoir une abstraction pour migrer vers un backend GPU plus tard.

### Modèle d’événements
- **Décision** : adopter un bus d’événements unidirectionnel UI -> core -> rendu.
- **Raison** : limiter le couplage et faciliter les tests.
- **Impact** : structurer les commandes et les événements dès le MVP.

## Interfaces clés (préliminaires)
- `EditorCore` : expose les opérations d’édition et l’état de sélection.
- `Renderer` : prend un snapshot du core et retourne un rendu du viewport.
- `Command` : encapsule une action utilisateur (insert, delete, move, search).
- `Event` : notification interne (document modifié, viewport changé).

## Stratégie de test
- Tests unitaires sur le core (buffer, sélection, undo/redo).
- Tests de performance ciblés (fichier volumineux, scroll rapide).
- Tests d’intégration sur le pipeline UI -> core -> render.
