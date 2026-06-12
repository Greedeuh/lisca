use std::collections::HashMap;
use std::path::Path;

/// Kokoro TTS model with built-in tokenizer.
pub struct KokoroModel {
    session: ort::session::Session,
    vocab: HashMap<char, i64>,
    voices: Vec<Vec<f32>>,  // Each voice is 256-dim style vector
    sample_rate: u32,
}

impl KokoroModel {
    /// Load a Kokoro model from an ONNX file and voice .bin file.
    pub fn load(model_path: &Path, voice_path: &Path) -> Result<Self, String> {
        if !model_path.exists() {
            return Err(format!("Model not found: {}", model_path.display()));
        }
        if !voice_path.exists() {
            return Err(format!("Voice not found: {}", voice_path.display()));
        }

        // Create ORT session
        let session = super::session::create_session(model_path)
            .map_err(|e| format!("Session: {}", e))?;

        // Load vocabulary
        let vocab = Self::load_vocab();

        // Load voice embeddings
        let voices = Self::load_voice(voice_path)?;

        Ok(Self {
            session,
            vocab,
            voices,
            sample_rate: 24000,
        })
    }

    /// Load the Kokoro vocabulary (phoneme → token ID).
    fn load_vocab() -> HashMap<char, i64> {
        let mut vocab = HashMap::new();
        // Generated from hexgrad/Kokoro-82M config.json
        let pairs = &[
            (';', 1), (':', 2), (',', 3), ('.', 4), ('!', 5), ('?', 6),
            ('—', 9), ('…', 10), ('"', 11), ('(', 12), (')', 13),
            ('\u{201C}', 14), ('\u{201D}', 15), (' ', 16),
            ('\u{0303}', 17), ('ʣ', 18), ('ʥ', 19), ('ʦ', 20), ('ʨ', 21),
            ('ᵝ', 22), ('\u{AB67}', 23),
            ('A', 24), ('I', 25), ('O', 31), ('Q', 33), ('S', 35),
            ('T', 36), ('W', 39), ('Y', 41), ('ᵊ', 42),
            ('a', 43), ('b', 44), ('c', 45), ('d', 46), ('e', 47),
            ('f', 48), ('h', 50), ('i', 51), ('j', 52), ('k', 53),
            ('l', 54), ('m', 55), ('n', 56), ('o', 57), ('p', 58),
            ('q', 59), ('r', 60), ('s', 61), ('t', 62), ('u', 63),
            ('v', 64), ('w', 65), ('x', 66), ('y', 67), ('z', 68),
            ('ɑ', 69), ('ɐ', 70), ('ɒ', 71), ('æ', 72), ('β', 75),
            ('ɔ', 76), ('ɕ', 77), ('ç', 78), ('ɖ', 80), ('ð', 81),
            ('ʤ', 82), ('ə', 83), ('ɚ', 85), ('ɛ', 86), ('ɜ', 87),
            ('ɟ', 90), ('ɡ', 92), ('ɥ', 99), ('ɨ', 101), ('ɪ', 102),
            ('ʝ', 103), ('ɯ', 110), ('ɰ', 111), ('ŋ', 112), ('ɳ', 113),
            ('ɲ', 114), ('ɴ', 115), ('ø', 116), ('ɸ', 118), ('θ', 119),
            ('œ', 120), ('ɹ', 123), ('ɾ', 125), ('ɻ', 126), ('ʁ', 128),
            ('ɽ', 129), ('ʂ', 130), ('ʃ', 131), ('ʈ', 132), ('ʧ', 133),
            ('ʊ', 135), ('ʋ', 136), ('ʌ', 138), ('ɣ', 139), ('ɤ', 140),
            ('χ', 142), ('ʎ', 143), ('ʒ', 147), ('ʔ', 148),
            ('ˈ', 156), ('ˌ', 157), ('ː', 158), ('ʰ', 162), ('ʲ', 164),
            ('↓', 169), ('→', 171), ('↗', 172), ('↘', 173), ('ᵻ', 177),
        ];
        for &(ch, id) in pairs {
            vocab.insert(ch, id);
        }
        vocab
    }

    /// Load voice embeddings from a .bin file.
    /// Format: raw f32 values, reshaped to (N, 256) where N >= max_token_len.
    fn load_voice(path: &Path) -> Result<Vec<Vec<f32>>, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Read voice: {}", e))?;
        if bytes.len() % 4 != 0 {
            return Err("Invalid voice file size".into());
        }

        let floats: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        // Reshape to (N, 256)
        if floats.len() % 256 != 0 {
            return Err(format!(
                "Voice file has {} floats, not divisible by 256",
                floats.len()
            ));
        }

        let n = floats.len() / 256;
        let voices = (0..n)
            .map(|i| floats[i * 256..(i + 1) * 256].to_vec())
            .collect();

        Ok(voices)
    }

    /// Convert text to token IDs using the vocabulary.
    ///
    /// This is a simplified tokenizer. For production, use misaki for
    /// proper phonemization. This handles basic ASCII and common phonemes.
    pub fn tokenize(&self, text: &str) -> Vec<i64> {
        let mut tokens = Vec::new();

        for ch in text.chars() {
            if let Some(&id) = self.vocab.get(&ch) {
                tokens.push(id);
            } else {
                // Try lowercase
                let lower = ch.to_lowercase().next().unwrap_or(ch);
                if let Some(&id) = self.vocab.get(&lower) {
                    tokens.push(id);
                }
                // Skip unknown characters
            }
        }

        tokens
    }

    /// Synthesize audio from text.
    pub fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String> {
        let tokens = self.tokenize(text);

        if tokens.is_empty() {
            return Err("No tokens generated from text".into());
        }

        // Max context length is 510 (512 - 2 for pad tokens)
        let tokens = if tokens.len() > 510 {
            &tokens[..510]
        } else {
            &tokens
        };

        // Select style vector based on token count
        let style_idx = (tokens.len() - 1).min(self.voices.len() - 1);
        let ref_s = &self.voices[style_idx];

        // Build input: add pad token (0) at start and end
        let mut input_ids = vec![0i64];
        input_ids.extend_from_slice(tokens);
        input_ids.push(0);

        // Create tensors
        let t_input_ids = ort::value::Tensor::from_array(([1, input_ids.len()], input_ids))
            .map_err(|e| format!("Tensor input_ids: {}", e))?;

        let t_style = ort::value::Tensor::from_array(([1, 1, 256], ref_s.clone()))
            .map_err(|e| format!("Tensor style: {}", e))?;

        let t_speed = ort::value::Tensor::from_array(([1], vec![speed]))
            .map_err(|e| format!("Tensor speed: {}", e))?;

        // Run inference
        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => t_input_ids.into_dyn(),
                "style" => t_style.into_dyn(),
                "speed" => t_speed.into_dyn(),
            ])
            .map_err(|e| format!("Inference: {}", e))?;

        // Extract audio
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Output: {}", e))?;

        eprintln!("Kokoro output shape: {:?}", shape);
        Ok(data.to_vec())
    }

    /// Get the model's sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
