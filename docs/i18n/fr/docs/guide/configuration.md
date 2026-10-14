<!-- roxanne-i18n: source=docs/guide/configuration.md sha=c36e8361227f3180dd74981fa7d096a7910a54bd -->

# Configuration

Roxanne lit ses réglages dans des fichiers TOML. La plupart figurent aussi sur
la page Préférences, qui les réécrit pour vous.

## Où sont les fichiers

Vos réglages vivent dans `Colony/Roxanne/`, sous le dossier de configuration de
la plateforme :

| Plateforme | Dossier |
|---|---|
| Linux | `~/.config/Colony/Roxanne/` (ou `$XDG_CONFIG_HOME/Colony/Roxanne/`) |
| Windows | `%LOCALAPPDATA%\Colony\Roxanne\` |
| macOS | `~/Library/Application Support/Colony/Roxanne/` |

Dans ce dossier :

- `config.toml` : votre configuration.
- `profiles/<nom>.toml` : les profils, que `profile = "<nom>"` dans
  `config.toml` charge par-dessus.

Un projet peut avoir son propre `.roxanne.toml`, trouvé dans le dossier de
travail ou l'un de ses parents.

Préférences → À propos indique le dossier sur votre machine.

### Migration depuis `~/.config/roxanne`

Roxanne gardait auparavant ses réglages dans `~/.config/roxanne/`. Au premier
démarrage avec le nouveau dossier, quand celui-ci n'a pas encore de
`config.toml`, Roxanne copie `config.toml` et `profiles/` depuis l'ancien et le
signale dans la barre d'état. Il écrit ensuite `.migrated-from-legacy-config`
dans le nouveau dossier, pour que la copie n'ait lieu qu'une fois. L'ancien
dossier reste exactement tel qu'il était : supprimez-le vous-même quand tout
vous semble correct. Si la copie échoue, la barre d'état dit pourquoi, rien
n'est marqué comme fait, et le démarrage suivant réessaie.

## Ordre

Chaque couche remplace la précédente, clé par clé :

1. Les valeurs par défaut intégrées.
2. Votre `config.toml`.
3. Le profil qu'il sélectionne, s'il y en a un.
4. Le `.roxanne.toml` du projet, puis le profil qu'il sélectionne.

Un fichier ne définit que les clés qu'il nomme. Un `[editor]` de projet avec
`tab_size = 2` change la largeur de tabulation et garde vos numéros de ligne,
votre retour à la ligne et votre minicarte. Une clé dont la valeur est invalide,
comme `tab_size = "two"`, est ignorée avec un avertissement et garde la valeur
que lui donnait la couche précédente ; les autres clés de ce fichier
s'appliquent quand même.

Un fichier de projet ne change jamais votre thème, vos autres préférences ni la
langue de l'interface, et ne charge jamais de code : `[preferences]`, `[theme]`
et `language` dans un `.roxanne.toml` sont ignorés avec un avertissement, tout
comme `plugins.dynamic` et `[lsp]`, qui permettraient à l'ouverture d'un dossier
de lancer un programme.

## La page Préférences

Cliquez sur **Roxanne** en haut à gauche ; cliquez à nouveau, ou sur Fermer,
pour revenir à l'éditeur. La page a les trois catégories par lesquelles
commence chaque programme Colony (Général, Apparence, Accessibilité), puis
Éditeur, Serveur de langage et Extensions, et À propos en dernier.

Il n'y a pas de bouton Enregistrer. Une modification s'applique aussitôt, et
seule la clé modifiée est écrite : dans votre profil quand le profil définit
cette clé, sinon dans votre `config.toml`. Une clé que le projet définit n'est
jamais écrite dans vos fichiers : sa ligne dans les Préférences nomme le fichier
du projet, la modification dure jusqu'au prochain rechargement des réglages, et
la barre d'état signale qu'elle n'a pas été enregistrée. Roxanne modifie un
fichier plutôt que de le réécrire, si bien que vos commentaires, l'ordre de vos
clés et les clés qu'il ne connaît pas sont tous conservés. Un fichier qui ne
s'analyse pas est laissé tel quel : la modification s'applique jusqu'à ce que
vous quittiez, et la barre d'état signale qu'elle n'a pas pu être enregistrée.

## Rechargement à chaud

**Outils → Recharger la configuration** relit les fichiers et applique ce qui a
changé.

Roxanne recharge aussi de lui-même quand `config.toml`, un fichier de
`profiles/` ou le `.roxanne.toml` du projet change sur le disque, est supprimé
ou déplacé, y compris un fichier créé après le démarrage de Roxanne. Il
surveille les dossiers qui contiennent ces fichiers, si bien qu'un éditeur qui
enregistre en renommant un nouveau fichier par-dessus l'ancien continue de
déclencher des rechargements. Un dossier `Colony/Roxanne/` ou `profiles/`
absent est pris en compte dès sa création, tant que le dossier au-dessus existe,
tout comme un dossier supprimé ou déplacé puis restauré. Sans configuration de
projet au démarrage, un nouveau `.roxanne.toml` est recherché dans le dossier de
travail, sauf si c'est le dossier personnel ou la racine d'un système de
fichiers : là, utilisez **Outils → Recharger la configuration**. Un
rechargement qui trouve les réglages déjà en vigueur, comme celui qui suit un
enregistrement par la page Préférences, ne fait rien.

Les fichiers ouverts sont surveillés de la même façon. Quand un autre programme
en modifie un, une barre au-dessus de l'éditeur propose de le recharger. Les
enregistrements de Roxanne lui-même ne demandent jamais rien.

## Exemple

```toml
config_version = 1
language = "fr"
keymap_profile = "vim"

[preferences]
selected_theme = "gruvbox"
selected_variant = "dark"
selected_accent = "violet"
auto_accent = false
font_size = "default"
text_size_a11y = "large"
high_contrast = false
dyslexia_font = false
reduce_motion = false
animations = true

[editor]
line_numbers = true
word_wrap = false
minimap = false

[lsp]
enabled = true
rust_analyzer = "rust-analyzer"

[keymap]
save = "cmd+s"
find_next = "f3"

[keymap.insert]
completion = "ctrl+space"

[keymap.normal]
enter_insert_mode = "i"

[keymap_profiles.vim]
enter_insert_mode = "i"
enter_normal_mode = "escape"

[plugins]
enabled = ["word_count", "line_count"]
dynamic = ["/opt/roxanne/plugins/libsample_plugin.so"]
```

## Clés

### Premier niveau

- `config_version` : la version du schéma, actuellement `1`. Un fichier qui ne
  la donne pas, ou qui en donne une plus ancienne, est migré en mémoire au
  chargement.
- `profile` : le profil à charger depuis `profiles/<profile>.toml`.
- `language` : `"en"` ou `"fr"`. Sans elle, Roxanne suit la langue du système
  (`LANGUAGE`, `LC_ALL`, `LC_MESSAGES` ou `LANG` sous Linux, la langue de
  l'utilisateur sous Windows et macOS), et revient à l'anglais à défaut.
- `keymap_profile` : le profil de `[keymap_profiles]` à utiliser.

### `[preferences]`

Dans vos propres fichiers uniquement. La page Préférences écrit cette table ;
les noms correspondent à ce que stocke chaque programme Colony.

- `selected_theme`, `selected_variant` : une famille de thèmes Colony et l'une
  de ses variantes, telles que Préférences → Apparence → Thème les liste. Un nom
  inconnu revient à Gruvbox sombre.
- `selected_accent` : l'une de `red`, `orange`, `yellow`, `green`, `blue`,
  `indigo`, `violet`, `amber`. Sans elle, l'accent propre au thème.
- `auto_accent` : `true` rend l'accent au thème.
- `font_size` : `"small"`, `"default"` ou `"large"`.
- `text_size_a11y` : `"small"`, `"default"`, `"large"` ou `"xlarge"`. Les deux
  tailles se multiplient.
- `high_contrast`, `dyslexia_font`, `reduce_motion`, `animations` : `true` ou
  `false`. Le code reste en JetBrainsMono Nerd Font quand la police pour
  dyslexiques est activée, car OpenDyslexic n'est pas à chasse fixe.

Une valeur que Roxanne ne comprend pas revient à sa valeur par défaut ; le reste
du fichier se charge quand même.

Avant les thèmes Colony, Roxanne avait une table `[theme]`. Dans votre propre
fichier, `name = "light"` est encore lu comme la variante claire du thème par
défaut ; ses couleurs personnalisées ne sont plus utilisées, et les charger
donne un avertissement.

### `[editor]`

- `line_numbers` : afficher les numéros de ligne. Ils sont masqués tant que
  `word_wrap` est activé, puisqu'une ligne repliée occupe plusieurs rangées.
- `word_wrap` : les longues lignes continuent sur la rangée suivante.
- `minimap` : afficher un aperçu du fichier à côté du texte.
- `tab_size`, `use_spaces`, `line_ending` : lus, pas encore appliqués.

### `[lsp]`

Dans vos propres fichiers uniquement.

- `enabled` : `false` arrête rust-analyzer et désactive les actions LSP.
- `rust_analyzer` : le programme à lancer, un nom cherché dans le `PATH` ou un
  chemin.

La prise en charge des serveurs de langage est expérimentale et ne couvre que
Rust. **Outils → Informations au survol (rust-analyzer)** et **Outils → Aller à
la définition (rust-analyzer)** fonctionnent sur un fichier `.rs` enregistré :
Roxanne démarre rust-analyzer à la première d'entre elles, dans le dossier le
plus proche au-dessus du fichier qui contient un `Cargo.lock`, sinon un
`Cargo.toml`, sinon le dossier du fichier lui-même. Interroger un fichier d'un
autre espace de travail remplace le serveur. La requête tourne en arrière-plan
et sa réponse s'affiche dans la barre d'état, ou ouvre la définition ; un
serveur qui ne répond pas à temps y affiche une erreur au lieu de figer
l'éditeur.

Roxanne demande à rust-analyzer de ne pas exécuter les scripts de build, les
macros procédurales ni `cargo check`, puisque tous trois exécutent du code du
projet. Il exécute tout de même `cargo metadata`, et les propres fichiers d'un
projet peuvent changer ce qu'il exécute : `rust-analyzer.toml`, la
configuration Cargo, et `rust-toolchain.toml`, qui peut désigner une chaîne
d'outils stockée dans le projet. N'utilisez les actions LSP que sur du code de
confiance.

### `[keymap]`

Clés : `save`, `open`, `find`, `find_next`, `find_previous`, `select_all`,
`copy`, `cut`, `paste`, `undo`, `redo`, `completion`, `completion_close`,
`enter_insert_mode`, `enter_normal_mode`.

`[keymap.insert]` et `[keymap.normal]` les remplacent pour un seul mode. Une
action qui n'appartient pas à un mode y est ignorée, avec un avertissement.

Un raccourci se compose de modificateurs et d'une touche joints par `+` :
`cmd`, `ctrl`, `shift`, `alt`, puis un caractère ou une touche nommée (`space`,
`escape`, `enter`, `tab`, `backspace`, `delete`, `home`, `end`, `pageup`,
`pagedown`, `insert`, les flèches `left`, `right`, `up`, `down`, et `f1` à
`f12`). Hors macOS, `cmd` et `ctrl` sont la même touche. Les menus affichent le
raccourci de chaque action dans les raccourcis en vigueur.

`[keymap_profiles.<nom>]` définit un ensemble réutilisable avec les mêmes clés,
choisi par `keymap_profile` ou dans Préférences → Éditeur → Raccourcis.

L'ancienne section `[keybindings]` est lue comme `[keymap]` ; les clés déjà
présentes dans `[keymap]` l'emportent.

### `[plugins]`

- `enabled` : les extensions intégrées à exécuter : `word_count`,
  `line_count`, `character_count`, `byte_count`, `longest_line`. Préférences →
  Extensions les active et les désactive.
- `dynamic` : les extensions natives (`.so`, `.dylib`, `.dll`), par chemin
  absolu, et uniquement dans vos propres fichiers (voir [Ordre](#ordre)) : un
  chemin relatif serait résolu par rapport au dossier de travail, il est donc
  refusé. Voir [plugins.md](plugins.md).
