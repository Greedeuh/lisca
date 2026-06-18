pub struct Transcriber;

impl Transcriber {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcriber_creation() {
        let _t = Transcriber::new();
    }
}
