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

## Seuils d'acceptation (Phase 4)
Les seuils initiaux sont définis dans `benchmarks/thresholds.toml` et sont
volontairement conservateurs. Ils sont à ajuster une fois la machine de
référence stabilisée.

Validation après exécution des benchmarks :
```bash
python scripts/check_benchmarks.py
```

## TODO (Phase 4)
- Intégrer les seuils d'acceptation en CI.
- Ajouter des scénarios de défilement/rendu quand le pipeline UI est benchable.
