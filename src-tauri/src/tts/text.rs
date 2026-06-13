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
