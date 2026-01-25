# Benchmarks – Scénarios & exécution

## Objectif
Poser une base reproductible pour mesurer la performance du core (TextBuffer),
avec des scénarios simples et comparables.

## Scénarios actuels
- Construction d'un TextBuffer à partir d'un texte de 1k, 10k et 50k lignes.
- Insertion d'un fragment au milieu d'un buffer de 10k lignes.
- Suppression d'un fragment fixe dans un buffer de 10k lignes.

## Exécution locale
```bash
cargo bench
```

## TODO (Phase 4)
- Définir des seuils d'acceptation et les intégrer en CI.
- Ajouter des scénarios de défilement/rendu quand le pipeline UI est benchable.
