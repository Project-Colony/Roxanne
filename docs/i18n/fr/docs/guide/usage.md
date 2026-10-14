<!-- roxanne-i18n: source=docs/guide/usage.md sha=ad7215a4d448a6e6d02658917ed088fbb469302c -->

# Utiliser Roxanne

Cette page reprend les libellés de l'interface en français, la langue que
Roxanne affiche quand votre système est en français ou que `language = "fr"`.

## La fenêtre

De haut en bas :

- **La barre de menus.** Le bouton **Roxanne** à gauche ouvre les Préférences,
  puis viennent Fichier, Édition, Sélection, Affichage, Atteindre, Outils, Projet
  et Aide. Chaque entrée affiche le raccourci qu'elle a dans les raccourcis en
  vigueur.
- **La barre d'onglets.** Un onglet par fichier ouvert. Un point après le nom
  signale des modifications non enregistrées ; le bouton `+` ouvre un nouvel
  onglet.
- **Les panneaux**, quand ils sont ouverts : rechercher et remplacer, aller à la
  ligne, diagnostics, complétions, Enregistrer sous, et la barre qui propose de
  recharger un fichier modifié par un autre programme.
- **L'éditeur**, avec l'arborescence des fichiers à sa gauche et la minicarte à
  sa droite quand vous les activez.
- **La barre d'état** : un champ de recherche rapide, le champ du nom de
  fichier, la position du curseur, le mode des raccourcis, les compteurs des
  extensions et le dernier message.

## Ouvrir et enregistrer des fichiers

Roxanne démarre avec un onglet vide et n'a pas de boîte de dialogue de fichiers.
Pour ouvrir un fichier, saisissez son chemin dans le champ du nom de fichier de
la barre d'état et choisissez **Fichier → Ouvrir le fichier…** (`Ctrl+O`,
`Cmd+O` sous macOS), ou cliquez dessus dans l'arborescence
(**Affichage → Afficher ou masquer l'arborescence**, `Ctrl+B`). Un fichier déjà
ouvert retrouve son onglet au lieu d'une seconde copie.

Les fichiers de plus de 5 Mio sont refusés. Les octets qui ne sont pas de
l'UTF-8 valide sont remplacés au chargement, et le premier enregistrement qui
suit vous demande d'enregistrer une seconde fois avant d'écrire le texte
remplacé.

**Fichier → Enregistrer** (`Ctrl+S`) écrit le fichier sur place. Un nouvel
onglet, ou un onglet dont vous avez modifié le nom de fichier, ouvre plutôt
Enregistrer sous, et Enregistrer sous demande avant de remplacer un fichier
existant. Chaque enregistrement passe par un fichier temporaire à côté du vrai,
qui le remplace ensuite en un seul renommage : le fichier n'est jamais laissé à
moitié écrit, il garde ses permissions, et un lien symbolique reste un lien.

Quand un autre programme modifie ou remplace un fichier ouvert, une barre
au-dessus de l'éditeur propose de le recharger. Un onglet qui a des
modifications non enregistrées les garde : Recharger laisse alors l'onglet tel
quel et la barre d'état le signale.

## Éditer

L'éditeur offre annuler et rétablir (`Ctrl+Z`, `Ctrl+Shift+Z` ou `Ctrl+Y`), qui
regroupent les modifications faites à moins d'une demi-seconde d'intervalle, et
les habituels couper, copier et coller.

**Sélection → Ajouter un curseur à l'occurrence suivante** et **Ajouter des
curseurs à toutes les occurrences** placent des curseurs supplémentaires sur les
occurrences du texte recherché ; la saisie, la suppression et le collage se
font alors à chaque curseur. **Effacer les curseurs** revient à un seul.

`Ctrl+Space` liste les complétions pour le mot qui précède le curseur : les
mots-clés et les types Rust, et les mots déjà présents dans le fichier.
`Escape` ferme la liste.

**Atteindre → Aller à la ligne…** accepte une ligne, ou `ligne:colonne`.

### Mode normal

Les raccourcis par défaut démarrent en mode insertion, où les touches saisissent
du texte. `Ctrl+[` passe à un mode normal expérimental inspiré de vim : `h`,
`j`, `k`, `l` déplacent le curseur, `w` et `b` le déplacent de mot en mot, `0`
va au début de la ligne, `g` et `G` au début et à la fin du fichier, `x` supprime
un caractère, `d` supprime la ligne, et `i`, `a`, `o` et `O` reviennent au mode
insertion. La barre d'état affiche le mode.

Les raccourcis ne voient que les touches que la zone de texte n'utilise pas
elle-même. Tant que la zone de texte a le focus clavier, les lettres saisissent
du texte dans les deux modes : appuyez sur `Escape` pour la quitter avant
d'utiliser le mode normal, et cliquez dans le texte pour saisir à nouveau.

Tous ces raccourcis peuvent être modifiés ; voir
[configuration.md](configuration.md#keymap).

## Rechercher

**Édition → Rechercher** (`Ctrl+F`) place le curseur dans le champ de recherche
de la barre d'état, qui cherche dans le fichier ouvert au fil de la saisie ;
`F3` et `Shift+F3` vont à l'occurrence suivante et précédente. **Édition →
Panneau de recherche** ouvre le panneau complet, avec les options de casse et
d'expressions régulières et le champ de remplacement.

**Remplacer** change l'occurrence courante et **Tout remplacer** toutes les
occurrences, dans le fichier ouvert uniquement. Avec les expressions régulières
activées, le remplacement peut utiliser des groupes de capture comme `$1`. Les
deux peuvent être annulés.

Réglez la portée sur l'espace de travail, ou choisissez **Projet → Rechercher
dans les fichiers…**, pour chercher dans tous les fichiers de l'espace de
travail : le dossier du `.roxanne.toml` le plus proche au-dessus du fichier
ouvert, ou à défaut le dossier du fichier lui-même. Cette recherche tourne en
arrière-plan et liste jusqu'à 500 occurrences ; cliquez sur l'une d'elles pour
l'ouvrir. Elle ne remplace pas. Les fichiers cachés sont ignorés sauf si vous
les incluez, tout comme `.git`, `target`, `node_modules`, `dist`, `build` et
`out`, les formats binaires courants, les fichiers de plus de 5 Mio et les
fichiers qui ne sont pas de l'UTF-8 valide.

## Intelligence du code

- **La coloration** avec tree-sitter pour Rust, C, Go, JavaScript, JSON,
  Markdown, Python et TOML, choisie d'après l'extension du fichier.
- **Le panneau des diagnostics** (**Édition → Panneau des diagnostics**) liste
  au fil de la saisie les délimiteurs qui ne s'apparient pas et les chaînes ou
  littéraux de caractère jamais terminés.
- **rust-analyzer**, pour les fichiers Rust uniquement : **Outils →
  Informations au survol (rust-analyzer)** et **Outils → Aller à la définition
  (rust-analyzer)**. Voir [configuration.md](configuration.md#lsp) pour la façon
  dont le serveur démarre et ce qu'il peut exécuter.

## Autres outils

- **Affichage → Palette de commandes** (`Ctrl+Shift+P`) liste toutes les
  actions ; tapez pour filtrer, cliquez pour lancer.
- **Affichage → Afficher ou masquer les numéros de ligne**, **Retour à la ligne
  automatique** et **Afficher ou masquer la minicarte** activent et désactivent
  ces éléments, et enregistrent le choix comme le fait Préférences → Éditeur.
  Tant que les Préférences sont ouvertes, les raccourcis clavier ne font rien.
- **Outils → Recharger la configuration** relit les fichiers de réglages.
- **Outils → Rapport de performance** affiche dans la barre d'état la durée du
  dernier démarrage, de la dernière ouverture, du dernier enregistrement, de la
  dernière recherche et du dernier rafraîchissement du viewport.

## Préférences

Cliquez sur **Roxanne** en haut à gauche ; la palette de commandes liste aussi
les Préférences. La page couvre le thème et l'accent Colony, la taille du texte,
le contraste élevé, la police OpenDyslexic, la réduction des animations, la
langue de l'interface, l'affichage de l'éditeur, le profil de raccourcis,
rust-analyzer et les extensions. Une modification s'applique aussitôt et est
écrite dans vos propres fichiers de réglages. Voir
[configuration.md](configuration.md#la-page-préférences).
