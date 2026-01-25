# Performance – Objectifs et mesures

## Objectifs
- Démarrage < 200 ms sur machine modeste.
- Navigation fluide sur fichiers 1-5 MB.
- RAM stable au repos.

## Mesures
- Temps de démarrage.
- FPS en défilement.
- Latence des commandes critiques.

## Mesures implémentées
- Démarrage, rendu (viewport), recherche et opérations I/O (ouverture/sauvegarde).
- Rapport rapide accessible via **Tools → Performance Report** (barre d'état).

## Benchmarks automatisés
Les benchmarks Criterion se trouvent dans `benches/` et couvrent :
- Construction de buffers sur fichiers moyens à très volumineux (`text_buffer_build`).
- Éditions répétées (insertion/suppression) et parcours de positions (`text_buffer_repeated_edits`).
- Rafales undo/redo pour valider l'historique (`text_buffer_history`).
- Mise à jour et réutilisation du cache de viewport (`viewport_cache_update`, `viewport_cache_reuse`).

### Lancer les benchmarks
1. Exécuter Criterion :
   ```bash
   cargo bench --bench text_buffer
   cargo bench --bench viewport
   ```
2. Vérifier les seuils :
   ```bash
   python3 scripts/check_benchmarks.py
   ```
   - Le script échoue si un benchmark manque, si un seuil est absent ou si un temps dépasse le seuil configuré.
   - Pour ignorer des résultats non référencés dans `benchmarks/thresholds.toml`, utiliser `--allow-unknown`.

### Mettre à jour les seuils
Après une mesure représentative sur la machine de référence :
1. Exécuter les benchmarks.
2. Reporter les temps moyens dans `benchmarks/thresholds.toml` (valeur `max_mean_ms`).
3. Relancer `scripts/check_benchmarks.py` pour valider.

## Stratégies
- Cache de rendu.
- Allocation mémoire maîtrisée.
- Profilage régulier.

## Planification
- Phase 1 : mesures de démarrage et de rendu sur un projet moyen.
- Phase 2 : seuils de latence pour la recherche et le highlight.
- Phase 4 : benchmarks automatisés en CI (scénarios TextBuffer prêts).
