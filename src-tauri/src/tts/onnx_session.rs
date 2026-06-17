use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use std::path::Path;

const DEFAULT_NUM_CPUS: usize = 4;

/// Creates an ONNX inference session with XNNPACK (CPU SIMD) or CPU fallback.
pub fn create_ort_model_session(path: &Path) -> Result<Session, ort::Error> {
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(DEFAULT_NUM_CPUS);

    // TODO: should we do this for any models, maybe model should chose how to create the session
    #[cfg(feature = "ort-xnnpack")]
    {
        eprintln!("Trying XNNPACK execution provider...");
        match try_create_session(path, num_cpus, "xnnpack") {
            Ok(session) => {
                eprintln!("XNNPACK session created");
                return Ok(session);
            }
            Err(e) => {
                eprintln!("XNNPACK failed: {}", e);
            }
        }
    }

    eprintln!("Using CPU execution provider");
    try_create_session(path, num_cpus, "cpu")
}

fn try_create_session(path: &Path, num_cpus: usize, provider: &str) -> Result<Session, ort::Error> {
    let mut builder =
        Session::builder()?.with_optimization_level(GraphOptimizationLevel::Level3)?;

    builder = builder
        .with_parallel_execution(true)?
        .with_intra_threads(num_cpus)?
        .with_inter_threads(1)?;

    match provider {
        #[cfg(feature = "ort-xnnpack")]
        "xnnpack" => {
            builder = builder.with_execution_providers([ort::ep::XNNPACK::default().build()])?;
        }
        _ => {
            builder = builder.with_execution_providers([ort::ep::CPU::default().build()])?;
        }
    }

    builder.commit_from_file(path)
}
