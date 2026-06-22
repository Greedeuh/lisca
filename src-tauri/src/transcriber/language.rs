// Language detection using whatlang, mapped to ISO 639-1 family codes.
// Returns None for empty input or when no installed languages match.

use whatlang::{Detector, Lang};

fn lang_to_family(lang: Lang) -> Option<&'static str> {
    match lang {
        Lang::Eng => Some("en"),
        Lang::Fra => Some("fr"),
        Lang::Deu => Some("de"),
        Lang::Spa => Some("es"),
        Lang::Ita => Some("it"),
        Lang::Por => Some("pt"),
        Lang::Nld => Some("nl"),
        Lang::Rus => Some("ru"),
        Lang::Pol => Some("pl"),
        Lang::Swe => Some("sv"),
        Lang::Dan => Some("da"),
        Lang::Fin => Some("fi"),
        Lang::Nob => Some("no"),
        Lang::Ces => Some("cs"),
        Lang::Hun => Some("hu"),
        Lang::Tur => Some("tr"),
        Lang::Ell => Some("el"),
        Lang::Ron => Some("ro"),
        Lang::Ukr => Some("uk"),
        Lang::Hin => Some("hi"),
        _ => None,
    }
}

fn family_to_lang(code: &str) -> Option<Lang> {
    match code {
        "en" => Some(Lang::Eng),
        "fr" => Some(Lang::Fra),
        "de" => Some(Lang::Deu),
        "es" => Some(Lang::Spa),
        "it" => Some(Lang::Ita),
        "pt" => Some(Lang::Por),
        "nl" => Some(Lang::Nld),
        "ru" => Some(Lang::Rus),
        "pl" => Some(Lang::Pol),
        "sv" => Some(Lang::Swe),
        "da" => Some(Lang::Dan),
        "fi" => Some(Lang::Fin),
        "no" => Some(Lang::Nob),
        "cs" => Some(Lang::Ces),
        "hu" => Some(Lang::Hun),
        "tr" => Some(Lang::Tur),
        "el" => Some(Lang::Ell),
        "ro" => Some(Lang::Ron),
        "uk" => Some(Lang::Ukr),
        "hi" => Some(Lang::Hin),
        _ => None,
    }
}

pub(crate)  fn detect_language_family(text: &str, installed_langs: &[String]) -> Option<&'static str> {
    let langs: Vec<Lang> = installed_langs.iter().filter_map(|s| family_to_lang(s)).collect();
    if langs.is_empty() {
        return None;
    }
    let detector = Detector::with_allowlist(langs);
    let lang = detector.detect_lang(text)?;
    lang_to_family(lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_langs() -> Vec<String> {
        vec![
            "en", "fr", "de", "es", "it", "pt", "nl", "ru", "pl", "sv",
            "da", "fi", "no", "cs", "hu", "tr", "el", "ro", "uk", "hi",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[test]
    fn detects_english() {
        assert_eq!(
            detect_language_family("Hello, world! This is a test.", &all_langs()),
            Some("en")
        );
    }

    #[test]
    fn detects_french() {
        assert_eq!(
            detect_language_family("Bonjour, comment allez-vous aujourd'hui?", &all_langs()),
            Some("fr")
        );
    }

    #[test]
    fn detects_german() {
        assert_eq!(
            detect_language_family("Guten Tag, wie geht es Ihnen?", &all_langs()),
            Some("de")
        );
    }

    #[test]
    fn detects_spanish() {
        assert_eq!(
            detect_language_family("Hola, ¿cómo estás? Espero que bien.", &all_langs()),
            Some("es")
        );
    }

    #[test]
    fn detects_italian() {
        assert_eq!(
            detect_language_family("Buongiorno, come stai oggi?", &all_langs()),
            Some("it")
        );
    }

    #[test]
    fn detects_russian() {
        assert_eq!(
            detect_language_family("Привет, как дела? Сегодня хорошая погода.", &all_langs()),
            Some("ru")
        );
    }

    #[test]
    fn detects_symptomes() {
        let fr_only = vec!["fr".to_string()];
        assert_eq!(detect_language_family("symptômes", &fr_only), Some("fr"));
    }

    #[test]
    fn returns_none_for_empty() {
        assert_eq!(detect_language_family("", &all_langs()), None);
    }

    #[test]
    fn returns_none_for_no_installed_langs() {
        assert_eq!(detect_language_family("Hello world", &[]), None);
    }

    #[test]
    fn family_to_lang_roundtrip() {
        for code in &["en", "fr", "de", "es", "it", "pt", "nl", "ru", "pl", "sv",
                       "da", "fi", "no", "cs", "hu", "tr", "el", "ro", "uk", "hi"] {
            let lang = family_to_lang(code).unwrap();
            assert_eq!(lang_to_family(lang), Some(*code));
        }
    }

    #[test]
    fn family_to_lang_rejects_unknown() {
        assert_eq!(family_to_lang("xx"), None);
        assert_eq!(family_to_lang(""), None);
    }
}
