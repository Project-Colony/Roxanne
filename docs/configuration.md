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
- Workspace : `.roxanne.toml` à la racine du projet

## Exemple minimal (TOML)
```toml
[theme.palette]
app_background = "#1e1e20"
status_bar = "#2d2d30"

[keymap]
save = "cmd+s"
find_next = "f3"

[plugins]
enabled = ["word_count", "line_count"]
```
