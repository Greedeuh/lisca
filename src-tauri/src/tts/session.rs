use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::path::Path;

/// Create an ONNX session with standard settings.
pub fn create_session(path: &Path) -> Result<Session, ort::Error> {
    let mut builder =
        Session::builder()?.with_optimization_level(GraphOptimizationLevel::Level3)?;

    builder = builder.with_parallel_execution(true)?;

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
