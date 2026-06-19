use std::collections::HashMap;

/// Basic English phonemizer (simplified).
/// Converts text to IPA phonemes for Kokoro TTS.
pub struct Phonemizer {
    word_map: HashMap<String, String>,
}

impl Phonemizer {
    pub fn new() -> Self {
        let mut word_map = HashMap::new();

        // Common English words -> IPA
        word_map.insert("the".into(), "ðə".into());
        word_map.insert("a".into(), "ə".into());
        word_map.insert("an".into(), "ən".into());
        word_map.insert("is".into(), "ɪz".into());
        word_map.insert("are".into(), "ɑːr".into());
        word_map.insert("was".into(), "wʌz".into());
        word_map.insert("were".into(), "wɜːr".into());
        word_map.insert("have".into(), "hæv".into());
        word_map.insert("has".into(), "hæz".into());
        word_map.insert("had".into(), "hæd".into());
        word_map.insert("do".into(), "duː".into());
        word_map.insert("does".into(), "dʌz".into());
        word_map.insert("did".into(), "dɪd".into());
        word_map.insert("will".into(), "wɪl".into());
        word_map.insert("would".into(), "wʊd".into());
        word_map.insert("can".into(), "kæn".into());
        word_map.insert("could".into(), "kʊd".into());
        word_map.insert("should".into(), "ʃʊd".into());
        word_map.insert("i".into(), "aɪ".into());
        word_map.insert("you".into(), "juː".into());
        word_map.insert("he".into(), "hiː".into());
        word_map.insert("she".into(), "ʃiː".into());
        word_map.insert("it".into(), "ɪt".into());
        word_map.insert("we".into(), "wiː".into());
        word_map.insert("they".into(), "ðeɪ".into());
        word_map.insert("this".into(), "ðɪs".into());
        word_map.insert("that".into(), "ðæt".into());
        word_map.insert("what".into(), "wʌt".into());
        word_map.insert("which".into(), "wɪtʃ".into());
        word_map.insert("who".into(), "huː".into());
        word_map.insert("how".into(), "haʊ".into());
        word_map.insert("not".into(), "nɒt".into());
        word_map.insert("no".into(), "noʊ".into());
        word_map.insert("yes".into(), "jɛs".into());
        word_map.insert("and".into(), "ænd".into());
        word_map.insert("or".into(), "ɔːr".into());
        word_map.insert("but".into(), "bʌt".into());
        word_map.insert("if".into(), "ɪf".into());
        word_map.insert("then".into(), "ðɛn".into());
        word_map.insert("so".into(), "soʊ".into());
        word_map.insert("very".into(), "vɛri".into());
        word_map.insert("my".into(), "maɪ".into());
        word_map.insert("your".into(), "jɔːr".into());
        word_map.insert("his".into(), "hɪz".into());
        word_map.insert("her".into(), "hɜːr".into());
        word_map.insert("our".into(), "aʊər".into());
        word_map.insert("their".into(), "ðɛər".into());
        word_map.insert("at".into(), "æt".into());
        word_map.insert("in".into(), "ɪn".into());
        word_map.insert("on".into(), "ɒn".into());
        word_map.insert("to".into(), "tuː".into());
        word_map.insert("for".into(), "fɔːr".into());
        word_map.insert("with".into(), "wɪð".into());
        word_map.insert("from".into(), "frɒm".into());
        word_map.insert("of".into(), "ʌv".into());
        word_map.insert("about".into(), "əbaʊt".into());
        word_map.insert("into".into(), "ɪntuː".into());
        word_map.insert("through".into(), "θruː".into());
        word_map.insert("before".into(), "bɪfɔːr".into());
        word_map.insert("after".into(), "ɑːftər".into());
        word_map.insert("between".into(), "bɪtwiːn".into());
        word_map.insert("under".into(), "ʌndər".into());
        word_map.insert("over".into(), "oʊvər".into());
        word_map.insert("hello".into(), "həˈloʊ".into());
        word_map.insert("hi".into(), "haɪ".into());
        word_map.insert("goodbye".into(), "ɡʊdˈbaɪ".into());
        word_map.insert("thanks".into(), "θæŋks".into());
        word_map.insert("please".into(), "pliːz".into());
        word_map.insert("sorry".into(), "sɒri".into());
        word_map.insert("love".into(), "lʌv".into());
        word_map.insert("like".into(), "laɪk".into());
        word_map.insert("want".into(), "wɒnt".into());
        word_map.insert("need".into(), "niːd".into());
        word_map.insert("go".into(), "ɡoʊ".into());
        word_map.insert("come".into(), "kʌm".into());
        word_map.insert("see".into(), "siː".into());
        word_map.insert("know".into(), "noʊ".into());
        word_map.insert("think".into(), "θɪŋk".into());
        word_map.insert("say".into(), "seɪ".into());
        word_map.insert("tell".into(), "tɛl".into());
        word_map.insert("give".into(), "ɡɪv".into());
        word_map.insert("take".into(), "teɪk".into());
        word_map.insert("make".into(), "meɪk".into());
        word_map.insert("good".into(), "ɡʊd".into());
        word_map.insert("great".into(), "ɡreɪt".into());
        word_map.insert("new".into(), "njuː".into());
        word_map.insert("now".into(), "naʊ".into());
        word_map.insert("here".into(), "hɪər".into());
        word_map.insert("there".into(), "ðɛər".into());
        word_map.insert("time".into(), "taɪm".into());
        word_map.insert("day".into(), "deɪ".into());
        word_map.insert("way".into(), "weɪ".into());
        word_map.insert("man".into(), "mæn".into());
        word_map.insert("woman".into(), "wʊmən".into());
        word_map.insert("child".into(), "tʃaɪld".into());
        word_map.insert("world".into(), "wɜːrld".into());
        word_map.insert("life".into(), "laɪf".into());
        word_map.insert("death".into(), "dɛθ".into());
        word_map.insert("work".into(), "wɜːrk".into());
        word_map.insert("home".into(), "hoʊm".into());
        word_map.insert("house".into(), "haʊs".into());
        word_map.insert("car".into(), "kɑːr".into());
        word_map.insert("dog".into(), "dɒɡ".into());
        word_map.insert("cat".into(), "kæt".into());
        word_map.insert("book".into(), "bʊk".into());
        word_map.insert("food".into(), "fuːd".into());
        word_map.insert("water".into(), "wɔːtər".into());
        word_map.insert("air".into(), "ɛər".into());
        word_map.insert("fire".into(), "faɪər".into());
        word_map.insert("earth".into(), "ɜːrθ".into());
        word_map.insert("sun".into(), "sʌn".into());
        word_map.insert("moon".into(), "muːn".into());
        word_map.insert("star".into(), "stɑːr".into());
        word_map.insert("tree".into(), "triː".into());
        word_map.insert("flower".into(), "flaʊər".into());
        word_map.insert("one".into(), "wʌn".into());
        word_map.insert("two".into(), "tuː".into());
        word_map.insert("three".into(), "θriː".into());
        word_map.insert("four".into(), "fɔːr".into());
        word_map.insert("five".into(), "faɪv".into());
        word_map.insert("six".into(), "sɪks".into());
        word_map.insert("seven".into(), "sɛvən".into());
        word_map.insert("eight".into(), "eɪt".into());
        word_map.insert("nine".into(), "naɪn".into());
        word_map.insert("ten".into(), "tɛn".into());
        word_map.insert("hundred".into(), "hʌndrəd".into());
        word_map.insert("thousand".into(), "θaʊzənd".into());
        word_map.insert("million".into(), "mɪljən".into());

        Self { word_map }
    }

    /// Convert text to IPA phonemes.
    pub fn phonemize(&self, text: &str) -> String {
        let mut result = String::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            let lower = word.to_lowercase();
            let clean: String = lower.chars().filter(|c| c.is_alphabetic()).collect();

            if let Some(phonemes) = self.word_map.get(&clean) {
                result.push_str(phonemes);
            } else {
                // Fallback: use letter-by-letter approximation
                for ch in clean.chars() {
                    result.push_str(&self.letter_to_phoneme(ch));
                }
            }

            if i < words.len() - 1 {
                result.push(' ');
            }
        }

        result
    }

    fn letter_to_phoneme(&self, ch: char) -> String {
        match ch {
            'a' => "æ".into(),
            'b' => "b".into(),
            'c' => "k".into(),
            'd' => "d".into(),
            'e' => "ɛ".into(),
            'f' => "f".into(),
            'g' => "ɡ".into(),
            'h' => "h".into(),
            'i' => "ɪ".into(),
            'j' => "dʒ".into(),
            'k' => "k".into(),
            'l' => "l".into(),
            'm' => "m".into(),
            'n' => "n".into(),
            'o' => "ɒ".into(),
            'p' => "p".into(),
            'q' => "k".into(),
            'r' => "ɹ".into(),
            's' => "s".into(),
            't' => "t".into(),
            'u' => "ʌ".into(),
            'v' => "v".into(),
            'w' => "w".into(),
            'x' => "ks".into(),
            'y' => "j".into(),
            'z' => "z".into(),
            _ => String::new(),
        }
    }
}
