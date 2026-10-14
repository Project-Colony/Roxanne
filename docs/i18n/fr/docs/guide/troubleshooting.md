<!-- roxanne-i18n: source=docs/guide/troubleshooting.md sha=ee0335936c3e3bad50592cd05b1dd61bbad63bb7 -->

# Dépannage

## La compilation échoue

Roxanne nécessite Rust 1.90 ou plus récent. Vérifiez la version et mettez à
jour la chaîne d'outils :

```bash
rustc --version
rustup update stable
```

## Un fichier ne s'ouvre pas

La barre d'état dit pourquoi. Deux limites s'appliquent à chaque fichier :

- les fichiers de plus de 5 Mio sont refusés ;
- les octets qui ne sont pas de l'UTF-8 valide sont remplacés au chargement. Le
  fichier s'ouvre, et le premier enregistrement vous demande d'enregistrer une
  seconde fois avant d'écrire le texte remplacé, pour que rien ne change par
  accident.

Un chemin saisi dans le champ du nom de fichier est relatif au dossier depuis
lequel Roxanne a été lancé. En cas de doute, saisissez un chemin absolu.

## Un réglage n'a aucun effet

- Après avoir chargé les réglages, la barre d'état compte les avertissements :
  un fichier qui ne s'analyse pas, un profil manquant, une extension ou un thème
  inconnus, un raccourci qui ne s'analyse pas. Choisissez **Outils → Recharger
  la configuration** pour recharger les fichiers et voir le compte.
- Le `.roxanne.toml` d'un projet ne peut pas changer votre thème, vos autres
  préférences, la langue de l'interface, `plugins.dynamic` ni `[lsp]` : ces clés
  y sont ignorées, avec un avertissement. Mettez-les dans votre propre
  `config.toml`.
- `tab_size`, `use_spaces` et `line_ending` sont lus mais pas encore appliqués.

Préférences → À propos indique où se trouve votre `config.toml`.

## Les réglages n'ont pas été enregistrés

La page Préférences écrit dans votre `config.toml` et laisse tel quel un fichier
qui ne s'analyse pas. La modification s'applique quand même jusqu'à ce que vous
quittiez, et la barre d'état signale qu'elle n'a pas pu être enregistrée.
Corrigez l'erreur TOML, puis modifiez à nouveau le réglage.

## rust-analyzer ne répond pas

- Les actions LSP ne fonctionnent que sur des fichiers `.rs` enregistrés.
- rust-analyzer doit être installé et présent dans votre `PATH`, par exemple
  avec `rustup component add rust-analyzer`, ou bien réglez `lsp.rust_analyzer`
  dans votre `config.toml` sur son chemin complet.
- La première requête sur un gros espace de travail peut prendre du temps :
  rust-analyzer charge l'espace de travail avant de répondre. Un serveur qui ne
  répond pas à temps affiche une erreur dans la barre d'état ; redemandez une
  fois le chargement terminé.
- Préférences → Serveur de langage, ou `lsp.enabled = false`, le désactive.

## Une extension native ne se charge pas

Le chemin dans `plugins.dynamic` doit être absolu et listé dans votre propre
`config.toml`, pas dans le `.roxanne.toml` d'un projet. La bibliothèque doit
exporter `roxanne_plugin_api_v1` ; voir [plugins.md](plugins.md).
