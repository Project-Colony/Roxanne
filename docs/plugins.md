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

## Lifecycle
1. Chargement.
2. Initialisation.
3. Exécution d’actions.
4. Déchargement.
