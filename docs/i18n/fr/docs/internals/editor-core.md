<!-- roxanne-i18n: source=docs/internals/editor-core.md sha=702af4b6920cefc46fab620699cdbd8275efbab9 -->

# Cœur de l'éditeur

Le modèle de texte vit dans `src/editor/` : `buffer.rs` contient le texte et son
historique, `highlight.rs` et `syntax.rs` le colorent, et `viewport.rs` garde
les lignes autour du curseur. Cette page couvre le buffer et la façon dont la
fenêtre le modifie ; [rendering.md](rendering.md) couvre le reste.

## Deux copies du texte

Chaque onglet garde son texte en double :

- `content`, un `text_editor::Content` d'iced, est ce que le widget d'édition
  dessine et modifie. Il détient le curseur et la sélection.
- `buffer`, un `TextBuffer`, est ce que lit le reste de Roxanne : annuler et
  rétablir, rechercher et remplacer, les diagnostics, les complétions, le LSP et
  les extensions.

Après chaque modification, la fenêtre copie le texte du widget dans le buffer
(`TextBuffer::replace`, qui ne fait rien quand le texte n'a pas changé). Dans
l'autre sens, annuler, rétablir, remplacer et recharger un fichier écrivent
d'abord dans le buffer, puis reconstruisent `content` à partir de lui.

## `TextBuffer`

- Le texte est une `ropey::Rope`, si bien qu'insérer ou supprimer au milieu d'un
  gros fichier ne déplace pas le reste.
- `text()` renvoie tout le texte sous forme de `&str` depuis un cache `String`,
  reconstruit après chaque modification. Cette copie est le principal coût
  d'une modification sur un gros fichier ; le benchmark
  `text_buffer_repeated_edits` la surveille.
- `revision()` compte les modifications. Le cache de viewport la compare pour
  éviter du travail quand rien n'a changé.
- Une `Position` est une ligne et une colonne, toutes deux à partir de zéro. Les
  colonnes comptent des caractères, pas des octets. Le widget iced compte des
  octets, donc la fenêtre convertit à la frontière (`cursor_position`,
  `move_cursor_to`).
- `line_count()` compte un saut de ligne final comme une dernière ligne vide
  supplémentaire, comme l'éditeur l'affiche.

## Annuler et rétablir

L'historique est une pile d'instantanés de la rope. Cloner une `Rope` partage
ses nœuds, si bien qu'un instantané coûte peu de mémoire quelle que soit la
taille du fichier.

- Avant une modification, la fenêtre appelle `record_snapshot`, sauf si la
  modification précédente date de moins de 500 ms : une rafale de frappe forme
  une seule étape d'annulation.
- Un nouvel instantané vide la pile de rétablissement. Des instantanés
  consécutifs identiques ne sont stockés qu'une fois.
- L'historique garde les 200 dernières étapes par onglet.
- Charger un fichier efface l'historique de son onglet.

## Curseurs multiples

L'onglet garde les curseurs supplémentaires dans une liste de `Position`s, à
côté du curseur propre au widget. Tant que la liste n'est pas vide, une
modification ne va pas au widget : `ui/editing.rs` la transforme en une
opération par curseur (insertion, collage, entrée, retour arrière, suppression,
indentation), les trie par position, écarte celles qui chevaucheraient une
précédente, et construit le nouveau texte en une passe. Chaque curseur se
déplace ensuite de ce que les modifications qui le précèdent ont ajouté ou
retiré, et le widget est reconstruit à partir du résultat.

Les curseurs s'ajoutent sur les occurrences de recherche (**Sélection → Ajouter
un curseur à l'occurrence suivante**, **Ajouter des curseurs à toutes les
occurrences**), donc c'est le texte recherché qui décide où ils vont.
