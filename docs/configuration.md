# Configuration – Fichiers et préférences

## Principes
- Configuration explicite et lisible.
- Valeurs par défaut raisonnables.
- Possibilité de surcharge par profil.

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

## Exemple minimal (TOML)
```toml
profile = "work"

[theme.palette]
app_background = "#1e1e20"
status_bar = "#2d2d30"

[theme.syntax]
keyword = "#569cd6"
string = "#ce9178"

[keymap]
save = "cmd+s"
find_next = "f3"

[keymap.insert]
completion = "ctrl+space"

[keymap.normal]
enter_insert_mode = "i"

[plugins]
enabled = ["word_count", "line_count"]
dynamic = ["./plugins/roxanne_sample.so"]
```

## Clés disponibles
### Profil
- `profile` : nom du profil à charger (cherche `~/.config/roxanne/profiles/<profil>.toml`).

### Thèmes
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

### Plugins
Clés possibles dans `[plugins]` :
- `enabled` : liste des plugins internes à activer (`word_count`, `line_count`).
- `dynamic` : chemins vers des plugins dynamiques (`.so`, `.dylib`, `.dll`).

## Planification configuration
- Phase 3 : merge multi-niveaux fiable + validation des schémas.
- Phase 4 : migration automatique des versions de config.
