<!-- roxanne-i18n: source=docs/internals/architecture.md sha=a850129b1b4066b01913fd15371477f16a1c7a61 -->

# Architecture

Roxanne est une seule crate : une bibliothèque dans `src/lib.rs` qui contient
tout, et un petit `src/main.rs` qui répond à `--version` et ouvre la fenêtre. La
fenêtre suit le modèle d'iced : une struct d'état, un enum de messages, une
fonction `update` qui modifie l'état, et une fonction `view` qui le dessine.

## Modules

| Module | Ce dont il a la charge |
|---|---|
| `ui` | la fenêtre : `RoxanneApp`, ses `Message`s et leur traitement, découpés par zone (`editing`, `files`, `tabs`, `menu`, `keys`, `search_panel`, `panels`, `file_tree`, `editor_view`, `status_bar`, `preferences`, `style`) |
| `editor` | le modèle de texte : `TextBuffer` sur une rope avec l'historique d'annulation, le surligneur tree-sitter, et le cache de viewport. Voir [editor-core.md](editor-core.md) et [rendering.md](rendering.md) |
| `config` | le chargement des réglages TOML en couches (`mod.rs`), la réécriture des modifications des Préférences (`store.rs`), et la copie unique depuis `~/.config/roxanne` (`migrate.rs`) |
| `keymap` | l'analyse des raccourcis, les raccourcis par défaut, et la correspondance entre événements clavier et actions en mode insertion ou normal |
| `search` | la recherche dans le fichier ouvert, et la recherche dans l'espace de travail qui parcourt l'arborescence |
| `lsp` | le client rust-analyzer : une session par racine d'espace de travail, sur son propre thread |
| `diagnostics` | les vérifications locales des délimiteurs, des chaînes et des littéraux de caractère |
| `completion` | les complétions tirées des mots-clés Rust et des mots du fichier |
| `plugins` | les compteurs intégrés et le chargeur d'extensions natives |
| `file_ops` | la limite de taille et l'écriture atomique par laquelle passe chaque enregistrement |
| `watch` | la souscription qui signale les fichiers modifiés par d'autres programmes |
| `i18n` | les chaînes anglaises et françaises (`en.toml`, `fr.toml`), par-dessus les chaînes partagées de colony-ui |

L'apparence, les polices, le squelette des Préférences et les réglages de thème
et d'accent viennent de [colony-ui](https://crates.io/crates/colony-ui), la
crate du design system de Project Colony ; `ui::style` ne fait que projeter sa
palette sur les widgets de Roxanne.

## Le parcours d'une modification

1. Le widget d'édition qui a le focus transforme les touches qu'il connaît en
   actions d'édition, chacune signalée comme un `Message::Edit`. Les touches
   qu'aucun widget n'a utilisées arrivent à la fenêtre comme `Message::Key` et
   passent par les raccourcis : un raccourci devient une `KeyAction`, traitée
   comme l'entrée de menu du même nom.
2. Une action d'édition modifie le `text_editor::Content` de l'onglet, qui
   contient le texte, le curseur et la sélection que le widget dessine. Avec
   plusieurs curseurs, Roxanne applique plutôt la modification au texte
   lui-même.
3. Le `TextBuffer` de l'onglet reçoit le nouveau texte. Annuler, rétablir,
   rechercher, remplacer, les diagnostics et le LSP lisent le buffer, pas le
   widget.
4. `text_changed` met ensuite à jour ce qui dépend du texte : la copie du
   surligneur, les occurrences de recherche, les diagnostics, les extensions et
   le cache de viewport.

## Ce qui tourne hors du thread de l'interface

Le thread de l'interface n'attend jamais le disque, le système de fichiers ni
un autre processus :

- Ouvrir et enregistrer un fichier, ouvrir un résultat de recherche et la
  recherche dans l'espace de travail sont des `Task`s ; leur résultat revient
  sous forme de message.
- rust-analyzer tourne sur un thread détenu par sa `LspSession`. Une requête
  envoie une tâche à ce thread et reçoit un future en retour, si bien qu'un
  serveur lent ou bloqué laisse l'éditeur réactif.
- La surveillance des fichiers est une `Subscription` iced sur un observateur
  `notify`. Elle surveille les dossiers qui contiennent les fichiers plutôt que
  les fichiers, si bien qu'un enregistrement qui renomme un nouveau fichier
  par-dessus l'ancien reste visible. Roxanne note ses propres écritures, pour
  que ses enregistrements ne demandent jamais de recharger le fichier.

Les extensions font exception : leurs callbacks s'exécutent sur le thread de
l'interface.

## Réglages

`AppConfig::load` lit les valeurs par défaut intégrées, le `config.toml` de
l'utilisateur, le profil qu'il sélectionne, puis le `.roxanne.toml` du projet et
son profil, chaque couche par-dessus la précédente, clé par clé : une table
comme `[editor]` ne définit que les clés qu'elle nomme. `AppConfig::sources`
note quel fichier a défini chaque clé. Un fichier de projet est filtré avant
d'être appliqué : il ne peut définir ni les préférences ni la langue, et son
`plugins.dynamic` et son `[lsp]` sont écartés, si bien qu'ouvrir un dossier ne
charge ni n'exécute jamais de code qui en vient.

La page Préférences modifie le réglage en mémoire et l'applique. Ensuite,
`store::save_changes` compare les réglages avant et après et n'écrit que les
clés qui ont changé, chacune dans le fichier de l'utilisateur qui la définit (le
profil ou `config.toml`), sinon dans `config.toml`, avec `toml_edit`, qui
conserve les commentaires, l'ordre des clés et les clés inconnues. Une clé
qu'un fichier de projet définit n'est pas écrite. Un fichier de configuration
surveillé qui change sur le disque déclenche un rechargement ; un rechargement
qui trouve les réglages déjà en vigueur ne fait rien.

Les fichiers de l'utilisateur vivent dans `Colony/Roxanne/` sous le dossier de
configuration de la plateforme, tel que le résout le module `paths` de
colony-ui.

## Décisions

- **Une rope pour le texte.** `ropey` rend peu coûteuses les modifications au
  milieu de gros fichiers, et peu coûteuse la copie des instantanés
  d'annulation. Le buffer garde aussi tout le texte en cache dans une `String`
  pour les appelants qui ont besoin d'un `&str`, ce qui coûte une copie par
  modification ; les benchmarks surveillent ce coût.
- **Le text editor d'iced pour le dessin et la gestion du curseur,** avec un
  surligneur sur mesure, plutôt qu'un widget de texte écrit à la main.
- **Des enregistrements en un seul renommage.** Un enregistrement ne tronque
  jamais le vrai fichier, si bien qu'un plantage ou un disque plein ne peut pas
  le laisser à moitié écrit.
- **Ne jamais exécuter le code du projet par surprise.** Les réglages de projet
  ne peuvent ni charger d'extensions ni choisir le programme du serveur de
  langage, et rust-analyzer ne démarre que sur une action LSP, avec les scripts
  de build et les macros procédurales désactivés.
