use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::path::Path;

/// Create an ONNX session with optimized settings.
pub fn create_session(path: &Path) -> Result<Session, ort::Error> {
    let mut builder =
        Session::builder()?.with_optimization_level(GraphOptimizationLevel::Level3)?;

    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    builder = builder
        .with_parallel_execution(true)?
        .with_intra_threads(num_cpus)?
        .with_inter_threads(1)?;

    // Try GPU first, fall back to CPU
    #[cfg(feature = "ort-directml")]
    {
        eprintln!("Trying DirectML execution provider...");
        match builder.clone()
            .with_execution_providers([ort::ep::DirectML::default().build()])?
            .commit_from_file(path)
        {
            Ok(session) => {
                eprintln!("DirectML session created successfully");
                return Ok(session);
            }
            Err(e) => {
                eprintln!("DirectML failed: {}, falling back to CPU", e);
            }
        }
    }

    // CPU fallback
    eprintln!("Using CPU execution provider");
    builder = builder.with_execution_providers([ort::ep::CPU::default().build()])?;

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

    eprintln!("ORT session created with {} CPU threads", num_cpus);

    Ok(session)
}
