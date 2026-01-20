# LSP – Intégration future

## Objectifs
- Diagnostics en temps réel.
- Autocomplétion.
- Navigation (go-to definition, references).

## Architecture envisagée
- Adaptateur LSP par langage.
- Communication asynchrone.
- Cache des diagnostics.

## Décisions à venir
- Bibliothèque LSP Rust.
- Stratégies de redémarrage et gestion des crashs.

## Planification LSP
- Phase 2 : LSP-lite (diagnostics locaux, complétions simples).
- Phase 3 : prototype LSP réel pour un langage prioritaire.
- Phase 4 : durcissement (cache, relance, logs).
