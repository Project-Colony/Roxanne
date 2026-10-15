<!-- roxanne-i18n: source=docs/project/overview.md sha=c5fe804a74180ebc02985bc3500e65989be7c01c -->

# Pourquoi Roxanne existe

## Le problème

Éditer un fichier de code oblige d'ordinaire à choisir entre deux sortes
d'outils. Un IDE comprend le code, mais exige d'abord un projet, met du temps à
démarrer, et exécute les scripts de build et les macros du projet pour le
comprendre, c'est-à-dire du code que vous n'avez peut-être pas encore lu. Un
simple éditeur de texte ouvre tout instantanément mais ne sait rien de ce qu'il
affiche.

Roxanne se place entre les deux : un éditeur natif qui ouvre n'importe quel
fichier sans projet, colore le code, et ne fait appel à un serveur de langage
que lorsqu'on le lui demande.

## Ce que Roxanne promet

- **Réactif sur les gros fichiers.** Le texte vit dans une rope, la coloration
  ne réanalyse que ce qui a changé, et le travail lent (lecture, enregistrement,
  recherche dans l'espace de travail, rust-analyzer) ne tourne jamais sur le
  thread de l'interface. Des benchmarks avec des limites en CI protègent le
  cœur ; voir [../internals/performance.md](../internals/performance.md).
- **Sûr par défaut.** Ouvrir un dossier ne charge ni n'exécute jamais de code
  qui en vient, et un enregistrement ne peut jamais laisser un fichier à moitié
  écrit.
- **Le clavier d'abord, configurable.** Chaque action est dans un menu et dans
  la palette de commandes, les raccourcis peuvent être redéfinis et regroupés en
  profils, et les réglages vivent dans de simples fichiers TOML que la page
  Préférences modifie sans les réécrire.
- **Un programme Colony.** Roxanne partage les thèmes, les polices, les options
  d'accessibilité et la disposition des Préférences de Project Colony avec les
  autres programmes de l'organisation, s'installe et se met à jour via
  [Colony](https://github.com/Project-Colony/Colony), et publie des versions
  signées.

## Objectifs

Ce sont des cibles que Roxanne vise, pas des mesures : rien ne les vérifie
encore. Les benchmarks de [../internals/performance.md](../internals/performance.md)
protègent le cœur de l'éditeur contre les régressions, ce qui n'est pas la même
chose.

- Démarrer en moins d'une seconde sur un projet de taille moyenne, avec une
  consommation mémoire raisonnable sur une machine modeste.
- Faire défiler et éditer un fichier de 50 000 lignes sans latence visible.

## Risques

- Un rendu de texte performant sous Linux, Windows et macOS est difficile.
  Roxanne le confie au widget d'édition de texte d'iced, si bien que ses limites
  sont celles de Roxanne.
- La fenêtre et le modèle de texte peuvent finir couplés si la frontière entre
  `ui` et `editor` s'estompe, ce qui rend les deux plus difficiles à tester et à
  faire évoluer. Les règles de modules de
  [CONTRIBUTING.md](../../CONTRIBUTING.md#structure-des-modules) sont là pour
  l'éviter.
- L'ergonomie au clavier demande plus de travail qu'il n'y paraît : les
  raccourcis, les modes et la façon dont le focus passe d'un panneau à l'autre
  exigent du soin pour rester prévisibles.

## À qui il s'adresse

- Aux développeurs qui veulent un éditeur rapide pour les fichiers de code et de
  configuration, à côté d'un IDE ou à sa place.
- À ceux qui lisent du code qu'ils ne connaissent pas, et veulent la coloration
  et la navigation sans exécuter ce code.
- À quiconque travaille surtout au clavier.

## Ce que Roxanne n'est pas

- Un IDE : pas de débogueur, pas d'intégration du système de build, pas de
  modèle de projet.
- Un éditeur de texte enrichi ou WYSIWYG.
- Un éditeur collaboratif ou web.

## Où il en est

Roxanne est en développement précoce. L'état indiqué en haut du
[README](../../README.md) dit ce qui est éprouvé et ce qui ne l'est pas encore,
et les [versions publiées](https://github.com/Project-Colony/Roxanne/releases)
disent ce qui a changé dans chacune.
