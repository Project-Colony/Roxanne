# Processus de release – Roxanne

Ce guide décrit une procédure reproductible pour publier une version stable
de Roxanne, du bump de version jusqu'aux artefacts distribuables.

## Pré-requis
- Avoir une branche main propre (tests verts, changelog à jour).
- Accès aux clés de signature si la distribution exige une signature.

## Étapes de release (checklist)
1. **Synchroniser la branche**
   - `git pull --rebase`
2. **Valider la qualité**
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all`
   - `cargo bench` (si applicable)
   - `python scripts/check_benchmarks.py`
3. **Mettre à jour la version**
   - Modifier la version dans `Cargo.toml`.
   - Mettre à jour le `CHANGELOG.md` (section *Unreleased* → version).
4. **Construire et packager**
   - `scripts/package.sh --target <triple>`
   - Optionnel : `SIGN=1 scripts/package.sh --target <triple>`
5. **Vérifier les artefacts**
   - `sha256sum -c dist/roxanne-<version>-<target>.tar.gz.sha256`
6. **Taguer la release**
   - `git commit -am "Release vX.Y.Z"`
   - `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
   - `git push origin main --tags`
7. **Publier**
   - Créer une release GitHub avec le changelog et les artefacts.

## Notes de version (template)
```
## Roxanne vX.Y.Z

### Ajouts
- ...

### Corrections
- ...

### Performance
- ...

### Documentation
- ...
```

## Rappels
- Garder le changelog strictement aligné au contenu de la release.
- Centraliser les artefacts dans `dist/` pour archivage.
