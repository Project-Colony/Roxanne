<!-- roxanne-i18n: source=docs/internals/rendering.md sha=a28c09fb1cb1ab188120f0d760a1e43af9a97c06 -->

# Rendu

Roxanne dessine son texte avec le widget `text_editor` d'iced. Le widget met en
page et peint le texte, le curseur et la sélection ; Roxanne lui fournit la
police, la taille, le mode de retour à la ligne et un surligneur. Le code de la
zone de texte se trouve dans `src/ui/editor_view.rs`.

## Polices et tailles

Le code est composé en JetBrainsMono Nerd Font, et le reste de l'interface dans
la police d'interface Colony, toutes deux embarquées par colony-ui et
enregistrées à l'ouverture de la fenêtre. La taille du code et la hauteur de
ligne suivent l'échelle de texte de l'utilisateur (`font_size` multiplié par
`text_size_a11y`), et la hauteur de ligne est une valeur absolue pour que les
numéros de ligne de la gouttière restent alignés sur leurs lignes. Avec la
police pour dyslexiques activée, l'interface passe à OpenDyslexic mais le code
reste à chasse fixe.

## Coloration

`editor::highlight::RoxanneHighlighter` implémente le trait `Highlighter`
d'iced. iced lui demande une ligne à la fois, à partir de la première ligne qui
a changé, et met le résultat en cache jusqu'à ce que les réglages changent.

- Le langage vient de l'extension du fichier (`Language::from_extension`). Le
  texte brut n'est pas coloré.
- Pour les huit langages pris en charge, `editor::syntax` garde un analyseur
  tree-sitter, la requête de coloration du langage et l'arbre syntaxique
  courant. Quand le texte change, il calcule l'unique plage modifiée entre
  l'ancien et le nouveau texte, l'applique à l'ancien arbre et réanalyse de
  façon incrémentale, si bien qu'une frappe ne réanalyse pas tout le fichier.
- Les couleurs d'une ligne sont les captures de la requête dans la plage
  d'octets de cette ligne, ramenées à quelques catégories : mot-clé, type,
  chaîne, commentaire et nombre.
- Les occurrences de recherche s'ajoutent par-dessus, dans leur propre
  catégorie.

Les couleurs viennent de la palette Colony active (`SyntaxColors::current`) :
les mots-clés prennent l'accent, les chaînes, les types et les nombres prennent
les couleurs d'état de la palette, les commentaires sa couleur de texte la plus
pâle. Les couleurs font partie des réglages du surligneur, si bien qu'un
changement de thème ou d'accent recolore toutes les lignes.

## Gouttière et minicarte

- La gouttière affiche les numéros des lignes visibles, d'après le décalage de
  défilement que la fenêtre suit à côté du widget. Elle est masquée pendant le
  retour à la ligne, puisqu'une ligne repliée occupe plusieurs rangées et que
  les numéros se décaleraient de leurs lignes.
- La minicarte échantillonne jusqu'à 60 lignes réparties uniformément sur le
  fichier et dessine un caractère bloc pour chacune, plus dense pour les lignes
  plus longues, avec un bloc plein à l'emplacement du curseur et le numéro de
  ligne du curseur en dessous.

## Le cache de viewport

`editor::ViewportCache` garde le texte des lignes autour du curseur, environ une
hauteur d'éditeur, et réutilise les lignes qu'il détient déjà quand la révision
du buffer n'a pas changé. Le widget ne dessine pas à partir de lui. Son temps de
rafraîchissement est ce que **Outils → Rapport de performance** appelle *Rendu*.
Les benchmarks `viewport` le mesurent sur 10 000 et 50 000 lignes ; voir
[performance.md](performance.md).
