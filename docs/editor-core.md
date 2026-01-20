# Editor Core – Buffer et opérations

## Concepts clés
- Buffer de texte central.
- Curseur et sélection.
- Historique d’édition (undo/redo).

## Opérations essentielles
- Insertion et suppression.
- Déplacement du curseur.
- Sélection (simple, multi-sélection future).

## MVP (cible initiale)
- Buffer de texte linéaire simple (`TextBuffer`) pour démarrer.
- Opérations de base : insertion, suppression par plage, undo/redo minimal.
- Synchronisation du buffer avec la recherche et les sauvegardes.

## Structures à comparer
- Rope.
- Gap buffer.
- Piece table.

## Décisions à venir
- Choix final de la structure.
- Limites de taille et performance.

## Planification core
- Phase 1 : stabiliser `TextBuffer` et les opérations fondamentales.
- Phase 2 : introduire multi-curseurs et sélections avancées.
- Phase 3 : exposer les hooks pour les plugins internes.
