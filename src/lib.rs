use dam::simulation::{Executed, InitializationOptionsBuilder, Initialized, ProgramBuilder};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

mod stream;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// A Python module implemented in Rust.
#[pymodule]
fn hoppy(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_class::<Program>()?;
    Ok(())
}

#[pyclass]
struct Program {
    state: ProgramState,
}

enum ProgramState {
    Building(ProgramBuilder<'static>),
    Initialized(Initialized<'static>),
    Executed(Executed<'static>),

    // Only used temporarily, for when the object is being actively used.
    Inconsistent,
}

impl ProgramState {
    fn initialize(&mut self, run_inference: bool) -> PyResult<()> {
        if let Self::Building(pb) = std::mem::replace(self, Self::Inconsistent) {
            let initialized = pb
                .initialize(
                    InitializationOptionsBuilder::default()
                        .run_flavor_inference(run_inference)
                        .build()
                        .unwrap(),
                )
                .map_err(|init_error| PyRuntimeError::new_err(init_error.to_string()))?;
            *self = Self::Initialized(initialized);
            return Ok(());
        }
        Err(PyRuntimeError::new_err(
            "Attempted to initialized a program that was not in the Building state.",
        ))
    }

    fn run(&mut self) -> PyResult<u64> {
        if let Self::Initialized(init) = std::mem::replace(self, Self::Inconsistent) {
            let executed = init.run(Default::default());
            let result = executed.elapsed_cycles().unwrap_or(0);

            *self = Self::Executed(executed);
            return Ok(result);
        }

        Err(PyRuntimeError::new_err(
            "Attempted to run a program that was not in the Initialized state.",
        ))
    }
}

#[pymethods]
impl Program {
    #[new]
    fn new() -> Self {
        Self {
            state: ProgramState::Building(ProgramBuilder::default()),
        }
    }

    #[pyo3(signature = (run_inference=false))]
    fn initialize(&mut self, run_inference: bool) -> PyResult<()> {
        self.state.initialize(run_inference)
    }

    fn run(&mut self) -> PyResult<u64> {
        self.state.run()
    }
}
