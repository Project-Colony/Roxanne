# Plugins – Extensibilité

## Objectifs
- API stable et documentée.
- Chargement dynamique.
- Isolation progressive (sandbox futur).

## Capacités envisagées
- Commandes personnalisées.
- Hooks d’événements (édition, rendu, UI).
- Extensions de syntaxe.

## Plugins internes initiaux (cibles)
- **word_count** : compteur de mots (status bar).
- **line_count** : compteur de lignes (status bar).
- **character_count** : compteur de caractères (status bar).
- **byte_count** : compteur d'octets (status bar).
- **longest_line** : longueur de la ligne la plus longue (status bar).

## Activation
Les plugins sont activés via la clé `plugins.enabled` dans la configuration TOML.
Plugins disponibles : `word_count`, `line_count`, `character_count`, `byte_count`, `longest_line`.
Les plugins dynamiques sont chargés via `plugins.dynamic` (chemins absolus ou relatifs).

## API dynamique (v1)
Les plugins dynamiques exposent un symbole `roxanne_plugin_api_v1` qui retourne une
structure avec des callbacks optionnels.

```rust
#[repr(C)]
struct PluginStatusV1 {
    label: *const c_char,
    value: *const c_char,
}

#[repr(C)]
struct PluginApiV1 {
    name: unsafe extern "C" fn() -> *const c_char,
    on_text_changed: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    on_file_opened: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    on_file_saved: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    status: Option<unsafe extern "C" fn() -> PluginStatusV1>,
}
```

Les chaînes C doivent rester valides tant que le plugin est chargé.

## Plugin externe (exemple)
Un plugin d'exemple est disponible dans `plugins/roxanne_sample`.

```bash
cargo build --release --manifest-path plugins/roxanne_sample/Cargo.toml
```

Le binaire dynamique produit se trouve dans :
- `target/release/libroxanne_sample.so` (Linux)
- `target/release/libroxanne_sample.dylib` (macOS)
- `target/release/roxanne_sample.dll` (Windows)

Ajoutez ensuite le chemin dans la config (ex : `plugins.dynamic`).

## Hooks actuels
- `on_text_changed` : après toute modification du contenu.
- `on_file_opened` : après ouverture d’un fichier.
- `on_file_saved` : après sauvegarde d’un fichier.

## Lifecycle
1. Chargement.
2. Initialisation.
3. Exécution d’actions.
4. Déchargement.

## Planification plugins
- Phase 3 : plugins internes + API minimaliste stable.
- Phase 4 : validation des interfaces et isolation progressive.
