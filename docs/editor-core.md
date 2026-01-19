# Editor Core – Buffer et opérations

## Concepts clés
- Buffer de texte central.
- Curseur et sélection.
- Historique d’édition (undo/redo).

## Opérations essentielles
- Insertion et suppression.
- Déplacement du curseur.
- Sélection (simple, multi-sélection future).

## MVP (implémentation actuelle)
- Buffer de texte linéaire simple (`TextBuffer`) utilisé par l’UI Iced.
- Opérations de base supportées : insertion, suppression par plage.
- Buffer synchronisé avec l’éditeur pour la recherche et les sauvegardes.

## Structures à comparer
- Rope.
- Gap buffer.
- Piece table.

## Décisions à venir
- Choix final de la structure.
- Limites de taille et performance.
