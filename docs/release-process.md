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
   - `sha256sum -c dist/roxanne-<version>-<target>.<ext>.sha256`
   - `sha256sum -c dist/SHA256SUMS`
   - Optionnel : `gpg --verify dist/roxanne-<version>-<target>.<ext>.asc dist/roxanne-<version>-<target>.<ext>`
6. **Taguer la release**
   - `git commit -am "Release vX.Y.Z"`
   - `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
   - `git push origin main --tags`
7. **Publier**
   - Créer une release GitHub avec le changelog et les artefacts.

## Artefacts attendus par OS
Chaque target produit un paquet `roxanne-<version>-<target>` contenant le binaire,
`README.md`, `LICENSE` et `installation.md`.

| OS | Format | Nom attendu |
| --- | --- | --- |
| Linux | `.tar.gz` | `roxanne-<version>-<target>.tar.gz` |
| macOS | `.tar.gz` | `roxanne-<version>-<target>.tar.gz` |
| Windows | `.zip` | `roxanne-<version>-<target>.zip` |

Fichiers de checksum/signature attendus (par artefact) :
- `roxanne-<version>-<target>.<ext>.sha256`
- `SHA256SUMS` (agrège toutes les sommes)
- `roxanne-<version>-<target>.<ext>.asc` (si `SIGN=1` et `gpg` disponible)
- `roxanne-<version>-<target>.<ext>.asc.sha256` (si signature générée)

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
