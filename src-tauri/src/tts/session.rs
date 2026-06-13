use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::path::Path;

/// Create an ONNX session with optimized settings.
/// Tries DirectML first, falls back to CPU if inference fails.
pub fn create_session(path: &Path) -> Result<Session, ort::Error> {
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Try DirectML first
    #[cfg(feature = "ort-directml")]
    {
        eprintln!("Trying DirectML execution provider...");
        if let Ok(mut session) = try_create_session(path, num_cpus, true) {
            eprintln!("DirectML session created, testing inference...");
            if test_inference(&mut session).is_ok() {
                eprintln!("DirectML inference OK");
                return Ok(session);
            }
            eprintln!("DirectML inference failed, falling back to CPU");
        }
    }

    // CPU fallback
    eprintln!("Using CPU execution provider");
    try_create_session(path, num_cpus, false)
}

fn try_create_session(path: &Path, num_cpus: usize, directml: bool) -> Result<Session, ort::Error> {
    let mut builder =
        Session::builder()?.with_optimization_level(GraphOptimizationLevel::Level3)?;

    builder = builder
        .with_parallel_execution(true)?
        .with_intra_threads(num_cpus)?
        .with_inter_threads(1)?;

    if directml {
        builder = builder.with_execution_providers([ort::ep::DirectML::default().build()])?;
    } else {
        builder = builder.with_execution_providers([ort::ep::CPU::default().build()])?;
    }

    let session = builder.commit_from_file(path)?;

    for input in session.inputs() {
        eprintln!(
            "TTS model input: name={}, type={:?}",
            input.name(),
            input.dtype()
        );
    }
    for output in session.outputs() {
        eprintln!(
            "TTS model output: name={}, type={:?}",
            output.name(),
            output.dtype()
        );
    }

    Ok(session)
}

/// Test inference with a dummy input to verify the session works.
fn test_inference(session: &mut Session) -> Result<(), String> {
    let t_input_ids = ort::value::Tensor::from_array(([1, 1], vec![0i64]))
        .map_err(|e| format!("Tensor: {}", e))?;
    let t_style = ort::value::Tensor::from_array(([1, 256], vec![0.0f32; 256]))
        .map_err(|e| format!("Tensor: {}", e))?;
    let t_speed = ort::value::Tensor::from_array(([1], vec![1.0f32]))
        .map_err(|e| format!("Tensor: {}", e))?;

    let _outputs = session
        .run(ort::inputs![
            "input_ids" => t_input_ids.into_dyn(),
            "style" => t_style.into_dyn(),
            "speed" => t_speed.into_dyn(),
        ])
        .map_err(|e| format!("Inference: {}", e))?;

    Ok(())
}
