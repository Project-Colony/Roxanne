# Benchmarks – Scénarios & exécution

## Objectif
Poser une base reproductible pour mesurer la performance du core (TextBuffer),
avec des scénarios simples et comparables.

## Scénarios actuels
- Construction d'un TextBuffer à partir d'un texte de 1k, 10k, 50k, 100k et 200k lignes.
- Insertion d'un fragment au milieu d'un buffer de 10k lignes.
- Suppression d'un fragment fixe dans un buffer de 10k lignes.
- Rafale undo/redo sur un buffer de 10k lignes (historique pré-rempli).
- Mise à jour du ViewportCache sur des buffers de 10k et 50k lignes, en simulant un défilement ligne par ligne.

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

Validation en CI (GitHub Actions) :
- Workflow `Benchmarks` exécutant `cargo bench` puis `scripts/check_benchmarks.py`.
