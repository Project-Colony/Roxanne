# Configuration: Fichiers et préférences

## Principes
- Configuration explicite et lisible.
- Valeurs par défaut raisonnables.
- Possibilité de surcharge par profil.
- Rechargement à chaud possible sans redémarrer l’application.

## Types de configuration
- Thèmes (couleurs, styles).
- Keymaps (raccourcis clavier).
- Options d’éditeur (tab size, wrap, etc.).
- Plugins (activation/désactivation des modules).

## Priorité
1. Valeurs par défaut.
2. Configuration utilisateur.
3. Configuration profil (optionnelle).
4. Configuration workspace.

## Emplacements
- Utilisateur : `~/.config/roxanne/config.toml`
- Profil : `~/.config/roxanne/profiles/<profil>.toml`
- Workspace : `.roxanne.toml` dans le workspace (recherche ascendante)

Native plugins (`plugins.dynamic`) are read only from the user configuration and the
profiles it selects. A workspace `.roxanne.toml`, and any profile it selects, cannot load
native code: its `plugins.dynamic` is ignored with a warning.

## Rechargement à chaud
- Utiliser **Tools → Reload Config** pour recharger les fichiers de configuration en cours
  d’exécution.
- Les thèmes, keymaps et plugins sont réappliqués immédiatement.

Roxanne also reloads on its own when `config.toml`, any file in `profiles/` or the
workspace `.roxanne.toml` changes on disk, is removed or is moved away, including a
file created after Roxanne started. It watches the directories that hold these files,
so an editor that saves by renaming a new file over the old one keeps triggering
reloads. A missing `~/.config/roxanne/` or `profiles/` directory is picked up once it
is created, as long as the directory above it exists, and so is one that is removed or
moved away and then restored. With no
workspace config at startup, a new `.roxanne.toml` is looked for in the working
directory, unless that is the home folder or a filesystem root: there, use
**Tools → Reload Config**.

Open files are watched the same way. A change made by another program asks whether to
reload the file. Roxanne's own saves never ask.

## Version du schéma
- `config_version` : entier optionnel qui indique la version du schéma de configuration.
- Si la version est absente ou inférieure à la version courante, Roxanne applique une migration
  automatique au chargement et écrit en mémoire la version courante.
- Version courante : `1`.

## Export / import des thèmes
- **Export** : utiliser **File → Export Theme** pour générer `.roxanne-theme.toml` dans le
  dossier courant.
- **Import** : utiliser **File → Import Theme** pour recharger `.roxanne-theme.toml` depuis
  le dossier courant.
- Le fichier exporté contient toutes les couleurs UI et syntaxe au format `#RRGGBB`.

## Exemple minimal (TOML)
```toml
config_version = 1
profile = "work"
keymap_profile = "vim"

[theme]
name = "dark"

[theme.palette]
app_background = "#1e1e20"
status_bar = "#2d2d30"

[theme.syntax]
keyword = "#569cd6"
string = "#ce9178"

[keymap]
save = "cmd+s"
find_next = "f3"

[keymap_profiles.vim]
enter_insert_mode = "i"
enter_normal_mode = "escape"

[keymap.insert]
completion = "ctrl+space"

[keymap.normal]
enter_insert_mode = "i"

[plugins]
enabled = ["word_count", "line_count"]
dynamic = ["/opt/roxanne/plugins/libroxanne_sample.so"]
```

## Clés disponibles
### Profil
- `profile` : nom du profil à charger (cherche `~/.config/roxanne/profiles/<profil>.toml`).

### Thèmes
Clé possible dans `[theme]` :
- `name` : thème prédéfini à charger (`default`, `dark`, `light`).

Clés possibles dans `[theme.palette]` (valeurs hexadécimales `#RRGGBB`) :
- `app_background`
- `menu_bar`
- `menu_button_active`
- `menu_button_hover`
- `submenu_bar`
- `button_base`
- `button_hover`
- `toggle_active`
- `toggle_inactive`
- `tab_bar`
- `tab_active`
- `editor_background`
- `status_bar`
- `panel_background`
- `panel_item_background`
- `panel_item_hover`

Clés possibles dans `[theme.syntax]` :
- `keyword`
- `type`
- `string`
- `comment`
- `number`
- `search_match`

### Keymaps
Clés possibles dans `[keymap]` :
- `save`
- `open`
- `find`
- `find_next`
- `find_previous`
- `select_all`
- `copy`
- `cut`
- `paste`
- `undo`
- `redo`
- `completion`
- `completion_close`
- `enter_insert_mode`
- `enter_normal_mode`

Les sections `[keymap.insert]` et `[keymap.normal]` permettent de surcharger par mode.
Les actions non applicables à un mode sont ignorées avec un avertissement.

Clés supplémentaires :
- `keymap_profile` : nom du profil à charger depuis `keymap_profiles`.
- `[keymap_profiles.<nom>]` : définit un profil de raccourcis réutilisable (mêmes clés que `[keymap]`).

Migration :
- La section `[keybindings]` est convertie en `[keymap]` lors du chargement.
- Les clés déjà définies dans `[keymap]` ont priorité sur `[keybindings]`.

### Plugins
Clés possibles dans `[plugins]` :
- `enabled` : liste des plugins internes à activer (`word_count`, `line_count`, `character_count`, `byte_count`, `longest_line`).
- `dynamic` : chemins vers des plugins dynamiques (`.so`, `.dylib`, `.dll`).
  Absolute paths only, and only in the user configuration (see above): a relative path
  would resolve against the working directory, so it is refused.
  Example: `/opt/roxanne/plugins/libroxanne_sample.so`, once an external plugin is built.

## Planification configuration
- Phase 3 : merge multi-niveaux fiable + validation des schémas.
- Phase 4 : migration automatique des versions de config.
