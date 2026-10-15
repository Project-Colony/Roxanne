<!-- roxanne-i18n: source=docs/internals/testing.md sha=bfb2e25f267c8a3eb66022830f61d7183b0c46e6 -->

# Tests

## Lancer les vérifications

La CI exécute ces commandes à chaque push et à chaque pull request ;
lancez-les avant d'en ouvrir une :

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

`cargo test` compile aussi `examples/sample_plugin.rs`, et
`tests/native_plugin.rs` charge la bibliothèque produite, si bien qu'une
modification de l'API des extensions ou de leur chargement qui casse l'exemple
fait échouer les tests. Ce test a besoin de l'exemple compilé : lancez-le via
`cargo test`, pas `cargo test --test native_plugin` seul. Un second job de CI
exécute `cargo check --all-targets` avec Rust 1.90, la `rust-version` de
`Cargo.toml`.

Pour les benchmarks et leurs limites, voir [performance.md](performance.md).

## Où sont les tests

**Les tests unitaires** se trouvent dans un `mod tests` en bas du module qu'ils
testent. L'interface a son propre fichier, `src/ui/tests.rs`, qui pilote
`RoxanneApp` par ses messages sans ouvrir de fenêtre. Ceux qui méritent d'être
connus :

- `editor/buffer.rs` : les positions sur de gros textes, l'insertion et la
  suppression, annuler et rétablir.
- `keymap.rs` : l'analyse et la correspondance des raccourcis, `cmd` et `ctrl`
  compris.
- `config/` : les couches clé par clé, la copie unique depuis
  `~/.config/roxanne`, et l'écriture des modifications des Préférences dans le
  bon fichier sans perdre les commentaires, sans toucher un fichier qui ne
  s'analyse pas et sans copier les réglages d'un projet.
- `lsp.rs` : le client face à un faux serveur sur un thread, si bien que les
  tests n'ont pas besoin de rust-analyzer.
- `watch.rs` : seuls les fichiers surveillés sont signalés, un dossier supprimé
  ou recréé reste surveillé, et les enregistrements de Roxanne lui-même sont
  ignorés là où c'est demandé.
- `i18n/mod.rs` : l'anglais et le français ont les mêmes clés et les mêmes
  paramètres `{name}`.
- `file_ops.rs` : un enregistrement qui échoue laisse le fichier d'origine tel
  qu'il était.
- `plugins.rs` : les chemins relatifs d'extensions natives sont refusés, et un
  nom d'extension inconnu donne un avertissement.

**Les tests d'intégration** de `tests/` n'utilisent que l'API publique de la
bibliothèque, ou le binaire :

| Fichier | Couvre |
|---|---|
| `atomic_write.rs` | un enregistrement conserve le mode du fichier et écrit à travers un lien symbolique |
| `cli.rs` | `--version` affiche la version sans ouvrir de fenêtre |
| `config_loading.rs` | les profils, les réglages propres à l'espace de travail, les valeurs par défaut de l'éditeur, les extensions natives refusées dans un projet, les chemins surveillés, une bascule enregistrée qui survit à un `[editor]` de projet |
| `config_migration.rs` | les anciennes sections `[keybindings]` deviennent `[keymap]` |
| `native_plugin.rs` | l'extension d'exemple se charge et son état arrive dans la barre d'état |

## Écrire des tests

- Les tests qui modifient `HOME`, `XDG_CONFIG_HOME` ou le dossier de travail
  tiennent un verrou pendant ce temps, comme `ENV_LOCK` dans
  `tests/config_loading.rs` : ces valeurs valent pour tout le processus et le
  harnais exécute les tests en parallèle.
- Utilisez `tempfile` pour tout ce qui touche au disque, jamais le vrai dossier
  de configuration.
- Une correction de bug s'accompagne d'un test qui échoue sans la correction.
- Rien dans la suite de tests n'a besoin d'un écran, d'une connexion réseau ni
  de rust-analyzer.
