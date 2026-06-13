use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::path::Path;

/// Create an ONNX session with optimized settings.
pub fn create_session(path: &Path) -> Result<Session, ort::Error> {
    let mut builder =
        Session::builder()?.with_optimization_level(GraphOptimizationLevel::Level3)?;

    // Use all available CPU cores for parallel execution
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    builder = builder
        .with_parallel_execution(true)?
        .with_intra_threads(num_cpus)? // Use all cores for matrix ops
        .with_inter_threads(1)?; // Single batch at a time

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

    eprintln!("ORT session created with {} threads", num_cpus);

    Ok(session)
}
