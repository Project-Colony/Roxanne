//! The interface strings, in English and French.
//!
//! Roxanne's own strings live in `en.toml` and `fr.toml` next to this file:
//! one flat `key = "text"` table per locale, English canonical. A `{name}` in a
//! string is filled in by [`t_fmt`]. The strings every Colony program shares
//! (the Preferences skeleton, theme and accent names) come from
//! [`colony_ui::i18n`], and so does the active locale: one [`set_locale`]
//! switches both.

use colony_ui::i18n as shared;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

pub use colony_ui::i18n::{Locale, locale, set_locale};

const EN: &str = include_str!("en.toml");
const FR: &str = include_str!("fr.toml");

fn table(locale: Locale) -> &'static HashMap<String, String> {
    static EN_TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    static FR_TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    let (cell, source) = match locale {
        Locale::En => (&EN_TABLE, EN),
        Locale::Fr => (&FR_TABLE, FR),
    };
    // Both files are compiled in and parsed by the tests below, so a panic
    // here means a build from a broken checkout.
    cell.get_or_init(|| toml::from_str(source).expect("locale file is a flat TOML table"))
}

/// The string for `key` in the active locale, else in English, else from the
/// shared Colony table, else the key itself, so a missing string shows up in
/// the interface instead of an empty label.
pub fn t(key: &str) -> &'static str {
    let locale = locale();
    table(locale)
        .get(key)
        .or_else(|| table(Locale::En).get(key))
        .map(String::as_str)
        .unwrap_or_else(|| shared::t_in(locale, key))
}

/// [`t`] with each `{name}` replaced by its value from `args`.
pub fn t_fmt(key: &str, args: &[(&str, &dyn Display)]) -> String {
    args.iter().fold(t(key).to_string(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), &value.to_string())
    })
}

/// The locale for the `language` setting: `"en"` or `"fr"`, or with no
/// setting, the system's language: `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` or
/// `LANG` on Linux and the BSDs, the user's locale on Windows and macOS.
pub fn resolve(language: Option<&str>) -> Locale {
    match language {
        Some(tag) => Locale::from_tag(tag),
        None => sys_locale::get_locale().map_or(Locale::En, |tag| Locale::from_tag(&tag)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn keys(locale: Locale) -> BTreeSet<&'static str> {
        table(locale).keys().map(String::as_str).collect()
    }

    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect()
    }

    #[test]
    fn fr_and_en_have_identical_key_sets() {
        let (en, fr) = (keys(Locale::En), keys(Locale::Fr));
        assert_eq!(
            en.difference(&fr).collect::<Vec<_>>(),
            Vec::<&&str>::new(),
            "missing in fr.toml"
        );
        assert_eq!(
            fr.difference(&en).collect::<Vec<_>>(),
            Vec::<&&str>::new(),
            "missing in en.toml"
        );
        for key in en {
            assert_eq!(
                placeholders(&table(Locale::En)[key]),
                placeholders(&table(Locale::Fr)[key]),
                "{key}: the two strings take different values"
            );
        }
    }

    /// Every literal key given to `t` or `t_fmt` in the sources names a string
    /// that exists, and every string in the locale files is used somewhere.
    #[test]
    fn every_key_in_the_sources_exists_and_every_string_is_used() {
        let call = regex::Regex::new(r#"\bt(?:_fmt)?\(\s*"([a-z0-9_]+)""#).expect("regex");
        let mut sources = String::new();
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in walkdir::WalkDir::new(&src)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.path().extension().is_some_and(|ext| ext == "rs") {
                sources.push_str(&std::fs::read_to_string(entry.path()).expect("source"));
            }
        }
        let en = table(Locale::En);
        for found in call.captures_iter(&sources) {
            let key = &found[1];
            assert!(
                en.contains_key(key) || shared::t_in(Locale::En, key) != key,
                "t(\"{key}\") has no string in en.toml or colony-ui"
            );
        }
        for key in en.keys() {
            assert!(
                sources.contains(&format!("\"{key}\"")),
                "{key} in en.toml is never used"
            );
        }
    }

    #[test]
    fn falls_back_to_the_shared_table_and_to_the_key() {
        assert_eq!(shared::t_in(Locale::En, "settings_title"), "Preferences");
        // Built at run time, so the check on literal keys above skips it.
        let missing = ["no_such", "key"].join("_");
        assert_eq!(t(&missing), missing);
    }

    #[test]
    fn formats_named_values() {
        let text = t_fmt("status_file_saved", &[("duration", &"1.0 ms")]);
        assert!(text.contains("1.0 ms") && !text.contains('{'), "{text}");
    }

    #[test]
    fn an_explicit_language_wins_over_the_system() {
        assert_eq!(resolve(Some("fr")), Locale::Fr);
        assert_eq!(resolve(Some("fr_FR.UTF-8")), Locale::Fr);
        assert_eq!(resolve(Some("en")), Locale::En);
    }
}
