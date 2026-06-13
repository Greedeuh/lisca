use std::collections::HashMap;
use std::path::Path;

use super::TtsBackend;

const MAX_TOKENS: usize = 510;
const VOICE_STYLE_DIM: usize = 256;
const KOKORO_SAMPLE_RATE: u32 = 24000;
const TOKEN_BOS: i64 = 0;
const TOKEN_EOS: i64 = 0;

/// Kokoro TTS model with misaki-rs phonemizer.
pub struct KokoroModel {
    session: ort::session::Session,
    vocab: HashMap<char, i64>,
    voices: Vec<Vec<f32>>,
    g2p: misaki_rs::G2P,
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

        let session = super::session::create_session(model_path)
            .map_err(|e| format!("Session: {}", e))?;

        let vocab = Self::load_vocab();
        let voices = Self::load_voices(voice_path)?;

        // Initialize misaki G2P for US English
        let g2p = misaki_rs::G2P::new(misaki_rs::Language::EnglishUS);

        let mut model = Self {
            session,
            vocab,
            voices,
            g2p,
        };

        // Warmup: run dummy inference to compile ONNX kernels
        // (session creation already tested inference for DirectML)
        eprintln!("Warming up model...");
        let start = std::time::Instant::now();
        match model.warmup() {
            Ok(()) => eprintln!("Model warmed up in {}ms", start.elapsed().as_millis()),
            Err(e) => eprintln!("Warmup failed (non-fatal): {}", e),
        }

        Ok(model)
    }

    /// Phoneme-to-ID mapping from the Kokoro model's vocab.txt.
    fn load_vocab() -> HashMap<char, i64> {
        let mut vocab = HashMap::new();
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

    fn load_voices(path: &Path) -> Result<Vec<Vec<f32>>, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Read voice: {}", e))?;
        if bytes.len() % 4 != 0 {
            return Err("Invalid voice file size".into());
        }

        let floats: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        if floats.len() % VOICE_STYLE_DIM != 0 {
            return Err(format!(
                "Voice file has {} floats, not divisible by {}",
                floats.len(),
                VOICE_STYLE_DIM
            ));
        }

        let n = floats.len() / VOICE_STYLE_DIM;
        let voices = (0..n)
            .map(|i| floats[i * VOICE_STYLE_DIM..(i + 1) * VOICE_STYLE_DIM].to_vec())
            .collect();

        Ok(voices)
    }

    fn tokenize(&self, text: &str) -> Vec<i64> {
        let (phonemes, _tokens) = self.g2p.g2p(text).unwrap_or_else(|e| {
            eprintln!("G2P error: {}", e);
            (text.to_string(), vec![])
        });

        let mut ids = Vec::new();
        for ch in phonemes.chars() {
            if let Some(&id) = self.vocab.get(&ch) {
                ids.push(id);
            }
        }
        ids
    }

    /// Run dummy inference to compile ONNX kernels ahead of time.
    fn warmup(&mut self) -> Result<(), String> {
        let input_ids_tensor = ort::value::Tensor::from_array(([1, 1], vec![0i64]))
            .map_err(|e| format!("Warmup tensor: {}", e))?;

        let style_tensor = ort::value::Tensor::from_array(([1, VOICE_STYLE_DIM], vec![0.0f32; VOICE_STYLE_DIM]))
            .map_err(|e| format!("Warmup tensor: {}", e))?;

        let speed_tensor = ort::value::Tensor::from_array(([1], vec![1.0f32]))
            .map_err(|e| format!("Warmup tensor: {}", e))?;

        let _outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => input_ids_tensor.into_dyn(),
                "style" => style_tensor.into_dyn(),
                "speed" => speed_tensor.into_dyn(),
            ])
            .map_err(|e| format!("Warmup inference: {}", e))?;

        Ok(())
    }
}

impl TtsBackend for KokoroModel {
    fn synthesize(&mut self, text: &str, speed: f32) -> Result<Vec<f32>, String> {
        let tokens = self.tokenize(text);

        if tokens.is_empty() {
            return Err("No tokens generated from text".into());
        }

        let tokens = if tokens.len() > MAX_TOKENS {
            &tokens[..MAX_TOKENS]
        } else {
            &tokens
        };

        let style_idx = (tokens.len() - 1).min(self.voices.len() - 1);
        let voice_style = &self.voices[style_idx];

        let mut input_ids = vec![TOKEN_BOS];
        input_ids.extend_from_slice(tokens);
        input_ids.push(TOKEN_EOS);

        let input_ids_tensor = ort::value::Tensor::from_array(([1, input_ids.len()], input_ids))
            .map_err(|e| format!("Tensor input_ids: {}", e))?;

        let style_tensor = ort::value::Tensor::from_array(([1, VOICE_STYLE_DIM], voice_style.clone()))
            .map_err(|e| format!("Tensor style: {}", e))?;

        let speed_tensor = ort::value::Tensor::from_array(([1], vec![speed]))
            .map_err(|e| format!("Tensor speed: {}", e))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input_ids" => input_ids_tensor.into_dyn(),
                "style" => style_tensor.into_dyn(),
                "speed" => speed_tensor.into_dyn(),
            ])
            .map_err(|e| format!("Inference: {}", e))?;

        let (_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Output: {}", e))?;

        Ok(data.to_vec())
    }

    fn sample_rate(&self) -> u32 {
        KOKORO_SAMPLE_RATE
    }
}
