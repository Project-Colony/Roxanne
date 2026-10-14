<!-- roxanne-i18n: source=docs/internals/performance.md sha=7780680b055d91fd93c9758c8eb4e6c74d0903b0 -->

# Performance

Deux outils surveillent la vitesse : le Rapport de performance dans l'éditeur,
et des benchmarks Criterion avec des limites que la CI fait respecter.

## Objectifs

Ce que Roxanne vise, pas ce qui a été mesuré : aucune vérification ne fait
encore respecter ces chiffres.

- Démarrage en moins de 200 ms sur une machine modeste.
- Défilement et édition fluides sur des fichiers de 1 à 5 Mo. Les fichiers plus
  gros sont refusés pour l'instant (la limite est de 5 Mio).
- Une consommation mémoire qui reste stable tant que l'éditeur est au repos.

## Le Rapport de performance

**Outils → Rapport de performance** affiche dans la barre d'état combien de
temps ces opérations ont pris la dernière fois :

| Mesure | Ce qui est chronométré |
|---|---|
| Démarrage | la construction de l'état de la fenêtre, extensions comprises |
| Rendu | le rafraîchissement du cache de viewport après une modification ou un défilement |
| Recherche | la recherche dans le fichier ouvert |
| Recherche dans les fichiers | la recherche dans l'espace de travail, de la demande aux résultats |
| Ouverture | la lecture d'un fichier, de la demande au texte dans son onglet |
| Enregistrement | l'écriture d'un fichier, de la demande à la confirmation |

Rien n'est enregistré ni envoyé nulle part ; les chiffres ne vivent que jusqu'à
la fermeture de Roxanne.

## Benchmarks

Les benchmarks de `benches/` mesurent le cœur de l'éditeur sans fenêtre :

| Groupe | Scénario |
|---|---|
| `text_buffer_build` | construire un `TextBuffer` de 1 000 à 200 000 lignes |
| `text_buffer_edit` | une insertion au milieu, et une suppression de 128 octets, dans 10 000 lignes |
| `text_buffer_repeated_edits` | 200 paires insertion et suppression dans 50 000 lignes, et le parcours des positions entre elles |
| `text_buffer_history` | 80 annulations puis 80 rétablissements dans 10 000 lignes |
| `viewport_cache_update` | déplacer un viewport de 60 lignes une ligne à la fois sur 10 000 et 50 000 lignes |
| `viewport_cache_reuse` | faire défiler un viewport de 80 lignes de 120 lignes vers le bas sur 100 000 lignes, si bien que la plupart des lignes sont déjà en cache |

Lancez-les, puis comparez les résultats aux limites :

```bash
cargo bench --bench text_buffer --bench viewport -- --noplot
python3 scripts/check_benchmarks.py
```

`scripts/check_benchmarks.py` lit les estimations de Criterion dans
`target/criterion/` et les limites dans `scripts/benchmark-thresholds.toml`, un
`max_mean_ms` (ou `max_mean_ns`) par benchmark. Il échoue quand un benchmark n'a
pas de résultat, quand un résultat n'a pas de limite, ou quand une moyenne
dépasse sa limite. `--allow-unknown` ignore les résultats sans limite,
`--thresholds` et `--criterion-dir` le dirigent vers d'autres fichiers.

Le workflow **Benchmarks** exécute les deux étapes à chaque pull request et à
chaque push sur `main`, sur un runner Ubuntu hébergé par GitHub.

## Changer une limite

Les limites sont fixées pour les runners de GitHub, plus lents et plus bruités
qu'un poste de bureau : chacune laisse de la marge au-dessus de l'exécution la
plus lente observée là-bas. Quand une modification rend un benchmark
légitimement plus lent, ou bien plus rapide, lancez les benchmarks plusieurs
fois, mettez à jour `max_mean_ms` dans `scripts/benchmark-thresholds.toml`, et
expliquez pourquoi dans la pull request. Un commentaire à côté d'une limite
indique comment elle a été choisie quand ce n'est pas évident.

## Coûts connus

- Chaque modification reconstruit le cache `String` du texte complet du buffer,
  si bien qu'une modification coûte un temps proportionnel à la taille du
  fichier. Le commentaire sur la limite `insert_delete_burst` indique ce que
  cela coûte en CI.
- La coloration réanalyse de façon incrémentale, mais le surligneur reçoit
  quand même une copie de tout le texte après chaque modification.
- La recherche dans l'espace de travail lit chaque fichier qu'elle n'ignore
  pas ; elle s'arrête à 500 occurrences.
