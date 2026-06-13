use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::{TensorElementType, ValueType};
use std::path::Path;

/// Create an ONNX session with optimized settings.
/// Tries DirectML first, falls back to CPU if inference fails.
pub fn create_session(path: &Path) -> Result<Session, ort::Error> {
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Try XNNPACK first (CPU SIMD acceleration)
    #[cfg(feature = "ort-xnnpack")]
    {
        eprintln!("Trying XNNPACK execution provider...");
        match try_create_session(path, num_cpus, "xnnpack") {
            Ok(mut session) => {
                eprintln!("XNNPACK session created, testing inference...");
                match test_inference(&mut session) {
                    Ok(()) => {
                        eprintln!("XNNPACK inference OK");
                        return Ok(session);
                    }
                    Err(e) => {
                        eprintln!("XNNPACK inference failed: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("XNNPACK session creation failed: {}", e);
            }
        }
    }

    // Try DirectML (GPU)
    #[cfg(feature = "ort-directml")]
    {
        eprintln!("Trying DirectML execution provider...");
        match try_create_session(path, num_cpus, "directml") {
            Ok(mut session) => {
                eprintln!("DirectML session created, testing inference...");
                match test_inference(&mut session) {
                    Ok(()) => {
                        eprintln!("DirectML inference OK");
                        return Ok(session);
                    }
                    Err(e) => {
                        eprintln!("DirectML inference failed: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("DirectML session creation failed: {}", e);
            }
        }
    }

    // CPU fallback
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
        #[cfg(feature = "ort-directml")]
        "directml" => {
            builder = builder.with_execution_providers([ort::ep::DirectML::default().build()])?;
        }
        #[cfg(feature = "ort-xnnpack")]
        "xnnpack" => {
            builder = builder.with_execution_providers([ort::ep::XNNPACK::default().build()])?;
        }
        _ => {
            builder = builder.with_execution_providers([ort::ep::CPU::default().build()])?;
        }
    }

    let session = builder.commit_from_file(path)?;

    for input in session.inputs() {
        eprintln!(
            "  input: name={}, type={:?}",
            input.name(),
            input.dtype()
        );
    }
    for output in session.outputs() {
        eprintln!(
            "  output: name={}, type={:?}",
            output.name(),
            output.dtype()
        );
    }

    Ok(session)
}

/// Test inference with a dummy input to verify the session works.
fn test_inference(session: &mut Session) -> Result<(), String> {
    let mut named_inputs: Vec<(std::borrow::Cow<str>, ort::session::SessionInputValue)> = Vec::new();
    for input in session.inputs() {
        let name = input.name().to_string();
        let dtype = input.dtype();
        let shape = match dtype {
            ValueType::Tensor { shape, .. } => shape.iter().map(|&d| if d < 0 { 1usize } else { d as usize }).collect::<Vec<_>>(),
            _ => return Err(format!("Input '{}' is not a tensor", name)),
        };
        let elem = match dtype {
            ValueType::Tensor { ty, .. } => *ty,
            _ => unreachable!(),
        };
        eprintln!("  test input: name={}, elem={:?}, shape={:?}", name, elem, shape);
        match elem {
            TensorElementType::Float32 => {
                let data: Vec<f32> = vec![0.0; shape.iter().product()];
                let tensor = ort::value::Tensor::from_array((shape.as_slice(), data))
                    .map_err(|e| format!("Tensor {}: {}", name, e))?;
                named_inputs.push((name.into(), tensor.into_dyn().into()));
            }
            TensorElementType::Int64 => {
                let data: Vec<i64> = vec![0; shape.iter().product()];
                let tensor = ort::value::Tensor::from_array((shape.as_slice(), data))
                    .map_err(|e| format!("Tensor {}: {}", name, e))?;
                named_inputs.push((name.into(), tensor.into_dyn().into()));
            }
            other => return Err(format!("Unsupported input dtype {:?} for '{}'", other, name)),
        }
    }

    let _outputs = session
        .run(named_inputs)
        .map_err(|e| format!("Inference: {}", e))?;

    Ok(())
}
