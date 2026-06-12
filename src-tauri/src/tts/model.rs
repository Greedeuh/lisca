use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;

use super::session::create_session;

/// ONNX-based TTS model.
pub struct TtsModel {
    session: Session,
    input_names: Vec<String>,
}

impl TtsModel {
    /// Load a TTS model from an ONNX file.
    pub fn load(model_path: &Path) -> Result<Self, String> {
        if !model_path.exists() {
            return Err(format!("Model not found: {}", model_path.display()));
        }

        let session = create_session(model_path).map_err(|e| format!("Session create: {}", e))?;

        let input_names: Vec<String> = session
            .inputs()
            .iter()
            .map(|i| i.name().to_string())
            .collect();

        Ok(Self {
            session,
            input_names,
        })
    }

    /// Synthesize audio from token IDs.
    ///
    /// # Arguments
    /// * `token_ids` - Tokenized text input
    /// * `speaker_id` - Optional speaker ID for multi-speaker models
    ///
    /// # Returns
    /// Audio samples as f32 values (PCM)
    pub fn synthesize(
        &mut self,
        token_ids: &[i64],
        speaker_id: Option<i64>,
    ) -> Result<Vec<f32>, String> {
        // Prepare input tensor
        let t_tokens = Tensor::from_array(([1, token_ids.len()], token_ids.to_vec()))
            .map_err(|e| format!("Tensor: {}", e))?;

        let mut inputs: Vec<(std::borrow::Cow<str>, ort::value::DynValue)> = vec![(
            self.input_names[0].as_str().into(),
            t_tokens.into_dyn(),
        )];

        // Add speaker ID if model expects it
        if let Some(sid) = speaker_id {
            if self.input_names.len() > 1 {
                let t_sid = Tensor::from_array(([1], vec![sid]))
                    .map_err(|e| format!("Tensor: {}", e))?;
                inputs.push((self.input_names[1].as_str().into(), t_sid.into_dyn()));
            }
        }

        // Run inference
        let outputs = self
            .session
            .run(inputs)
            .map_err(|e| format!("Inference: {}", e))?;

        // Extract audio samples
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Output: {}", e))?;

        eprintln!("TTS output shape: {:?}", shape);
        Ok(data.to_vec())
    }

    /// Get the model's expected sample rate from metadata.
    pub fn sample_rate(&self) -> u32 {
        self.session
            .metadata()
            .ok()
            .and_then(|m| m.custom("sample_rate").map(|s| s.to_string()))
            .and_then(|s| s.parse().ok())
            .unwrap_or(22050)
    }

    /// Get input names for debugging.
    pub fn input_names(&self) -> &[String] {
        &self.input_names
    }
}
