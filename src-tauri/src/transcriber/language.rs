use whatlang::Lang;

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

pub fn detect_language_family(text: &str) -> Option<&'static str> {
    let info = whatlang::detect(text)?;
    if info.confidence() < 0.1 {
        return None;
    }
    lang_to_family(info.lang())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_english() {
        assert_eq!(
            detect_language_family("Hello, world! This is a test."),
            Some("en")
        );
    }

    #[test]
    fn detects_french() {
        assert_eq!(
            detect_language_family("Bonjour, comment allez-vous aujourd'hui?"),
            Some("fr")
        );
    }

    #[test]
    fn detects_german() {
        assert_eq!(
            detect_language_family("Guten Tag, wie geht es Ihnen?"),
            Some("de")
        );
    }

    #[test]
    fn detects_spanish() {
        assert_eq!(
            detect_language_family("Hola, ¿cómo estás? Espero que bien."),
            Some("es")
        );
    }

    #[test]
    fn detects_italian() {
        assert_eq!(
            detect_language_family("Buongiorno, come stai oggi?"),
            Some("it")
        );
    }

    #[test]
    fn detects_russian() {
        assert_eq!(
            detect_language_family("Привет, как дела? Сегодня хорошая погода."),
            Some("ru")
        );
    }

    #[test]
    fn returns_none_for_empty() {
        assert_eq!(detect_language_family(""), None);
    }

    #[test]
    fn returns_none_for_too_short() {
        assert_eq!(detect_language_family("ok"), None);
    }

    #[test]
    fn lang_to_family_covers_known_languages() {
        assert_eq!(lang_to_family(Lang::Eng), Some("en"));
        assert_eq!(lang_to_family(Lang::Fra), Some("fr"));
        assert_eq!(lang_to_family(Lang::Deu), Some("de"));
        assert_eq!(lang_to_family(Lang::Spa), Some("es"));
        assert_eq!(lang_to_family(Lang::Ita), Some("it"));
        assert_eq!(lang_to_family(Lang::Por), Some("pt"));
        assert_eq!(lang_to_family(Lang::Nld), Some("nl"));
        assert_eq!(lang_to_family(Lang::Rus), Some("ru"));
        assert_eq!(lang_to_family(Lang::Pol), Some("pl"));
        assert_eq!(lang_to_family(Lang::Swe), Some("sv"));
        assert_eq!(lang_to_family(Lang::Dan), Some("da"));
        assert_eq!(lang_to_family(Lang::Fin), Some("fi"));
        assert_eq!(lang_to_family(Lang::Nob), Some("no"));
        assert_eq!(lang_to_family(Lang::Ces), Some("cs"));
        assert_eq!(lang_to_family(Lang::Hun), Some("hu"));
        assert_eq!(lang_to_family(Lang::Tur), Some("tr"));
        assert_eq!(lang_to_family(Lang::Ell), Some("el"));
        assert_eq!(lang_to_family(Lang::Ron), Some("ro"));
        assert_eq!(lang_to_family(Lang::Ukr), Some("uk"));
        assert_eq!(lang_to_family(Lang::Hin), Some("hi"));
    }
}
