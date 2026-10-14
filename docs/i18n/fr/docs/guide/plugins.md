<!-- roxanne-i18n: source=docs/guide/plugins.md sha=74fb312f35023a230a644080c01db26a693d86af -->

# Extensions

Les extensions (plugins) ajoutent des compteurs à la barre d'état. Il en existe
deux sortes : les extensions intégrées, livrées avec Roxanne, et les extensions
natives, des bibliothèques partagées que vous compilez ou téléchargez vous-même.

## Extensions intégrées

| Nom | Affiche |
|---|---|
| `word_count` | le nombre de mots |
| `line_count` | le nombre de lignes |
| `character_count` | le nombre de caractères |
| `byte_count` | la taille en octets |
| `longest_line` | la longueur de la ligne la plus longue |

Les cinq tournent par défaut. Activez-les et désactivez-les dans Préférences →
Extensions, ou listez celles que vous voulez dans `plugins.enabled` :

```toml
[plugins]
enabled = ["word_count", "line_count"]
```

Le `.roxanne.toml` d'un projet peut lui aussi définir `plugins.enabled`.

## Extensions natives

Une extension native est un `.so`, un `.dylib` ou un `.dll` que Roxanne charge
au démarrage, et à nouveau quand les réglages des extensions changent. Elle
tourne dans Roxanne avec vos droits, donc Roxanne n'en charge une que si toutes
ces conditions sont réunies :

- elle est listée dans `plugins.dynamic` dans votre propre `config.toml`, ou
  dans un profil que ce fichier sélectionne. `plugins.dynamic` dans le
  `.roxanne.toml` d'un projet est ignoré, si bien qu'ouvrir un dossier ne charge
  jamais de code depuis celui-ci ;
- son chemin est absolu. Un chemin relatif serait résolu par rapport au dossier
  de travail, qui peut être le projet que vous avez ouvert.

```toml
[plugins]
dynamic = ["/home/you/plugins/libsample_plugin.so"]
```

Sous Windows, les DLL dont dépend l'extension sont cherchées à côté de
l'extension, à côté de Roxanne et dans System32, jamais dans le dossier de
travail.

Préférences → Extensions liste les extensions natives utilisées. Une extension
qui ne se charge pas est ignorée, et la barre d'état la compte parmi les
avertissements de configuration.

## Écrire une extension native

Une extension native exporte une seule fonction C, `roxanne_plugin_api_v1`, qui
renvoie son nom et ses callbacks :

```rust
#[repr(C)]
pub struct PluginStatusV1 {
    pub label: *const c_char,
    pub value: *const c_char,
}

#[repr(C)]
pub struct PluginApiV1 {
    pub name: unsafe extern "C" fn() -> *const c_char,
    pub on_text_changed: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub on_file_opened: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub on_file_saved: Option<unsafe extern "C" fn(*const c_char, *const c_char)>,
    pub status: Option<unsafe extern "C" fn() -> PluginStatusV1>,
}
```

- `on_text_changed` s'exécute après chaque modification du texte,
  `on_file_opened` après l'ouverture d'un fichier et `on_file_saved` après son
  enregistrement. Chacun reçoit tout le texte et le nom du fichier en UTF-8
  terminé par un NUL ; un NUL à l'intérieur du texte est remplacé avant l'appel.
  Les pointeurs ne sont valides que pendant l'appel.
- `status` renvoie le libellé et la valeur affichés dans la barre d'état. Les
  deux chaînes doivent rester valides jusqu'au prochain appel à l'extension, et
  être en UTF-8.
- Chaque callback s'exécute sur le thread de l'interface : une extension lente
  ralentit l'éditeur.

[`examples/sample_plugin.rs`](../../../../../examples/sample_plugin.rs) est une
extension complète qui compte les mots et les lignes. Compilez-la depuis la
racine du dépôt :

```bash
cargo build --release --example sample_plugin
```

La bibliothèque arrive dans `target/release/examples/` : `libsample_plugin.so`
sous Linux, `libsample_plugin.dylib` sous macOS, `sample_plugin.dll` sous
Windows. Ajoutez son chemin absolu à `plugins.dynamic` dans votre propre
`config.toml`, dans `Colony/Roxanne/` sous votre dossier de configuration
(`~/.config/Colony/Roxanne/` sous Linux).
