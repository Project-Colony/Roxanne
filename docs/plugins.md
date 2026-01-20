# Plugins – Extensibilité

## Objectifs
- API stable et documentée.
- Chargement dynamique.
- Isolation progressive (sandbox futur).

## Capacités envisagées
- Commandes personnalisées.
- Hooks d’événements (édition, rendu, UI).
- Extensions de syntaxe.

## Plugins internes initiaux
- **word_count** : compteur de mots (status bar).
- **line_count** : compteur de lignes (status bar).

## Activation
Les plugins sont activés via la clé `plugins.enabled` dans la configuration TOML.
Plugins disponibles : `word_count`, `line_count`.
Les plugins dynamiques sont chargés via `plugins.dynamic` (chemins absolus ou relatifs).

## Hooks actuels
- `on_text_changed` : après toute modification du contenu.
- `on_file_opened` : après ouverture d’un fichier.
- `on_file_saved` : après sauvegarde d’un fichier.

## Lifecycle
1. Chargement.
2. Initialisation.
3. Exécution d’actions.
4. Déchargement.
