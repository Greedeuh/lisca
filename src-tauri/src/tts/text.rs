use std::sync::OnceLock;

pub fn split_text(text: &str) -> Vec<String> {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"([.!?;])\s+").unwrap());
    let mut chunks: Vec<String> = Vec::new();
    let mut last = 0;
    for m in re.find_iter(text) {
        let split_at = m.start() + 1;
        let chunk = text[last..split_at].trim().to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        last = m.end();
    }
    if last < text.len() {
        let tail = text[last..].trim().to_string();
        if !tail.is_empty() {
            chunks.push(tail);
        }
    }
    if chunks.is_empty() {
        vec![text.trim().to_string()]
    } else {
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_sentence() {
        assert_eq!(split_text("Hello world."), vec!["Hello world."]);
    }

    #[test]
    fn multiple_sentences() {
        assert_eq!(
            split_text("Hello. World? How are you?"),
            vec!["Hello.", "World?", "How are you?"]
        );
    }

    #[test]
    fn semicolons() {
        assert_eq!(
            split_text("First; second; third."),
            vec!["First;", "second;", "third."]
        );
    }

    #[test]
    fn empty_string() {
        assert_eq!(split_text(""), vec![""]);
    }

    #[test]
    fn no_punctuation() {
        assert_eq!(split_text("no punctuation here"), vec!["no punctuation here"]);
    }

    #[test]
    fn trailing_punctuation_no_space() {
        assert_eq!(split_text("Hello."), vec!["Hello."]);
    }

    #[test]
    fn multiple_punctuation_marks() {
        assert_eq!(
            split_text("Really? Yes! OK..."),
            vec!["Really?", "Yes!", "OK..."]
        );
    }

    #[test]
    fn whitespace_only() {
        assert_eq!(split_text("   "), vec![""]);
    }
}
