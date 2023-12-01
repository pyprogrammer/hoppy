use std::ffi::CString;

use dam::simulation::{Executed, InitializationOptionsBuilder, Initialized, ProgramBuilder};
use pyo3::prelude::*;
use pyo3::types::{PyCapsule, PyDict, PyTuple};
use pyo3::{exceptions::PyRuntimeError, types::PyList};
use stream::{HopStream, StreamEnum};

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

    /// Returns {
    ///     cycles: u64,
    ///     results: List<List<values>>
    /// }
    #[pyo3(signature = (outputs, *, run_inference=false))]
    fn run<'a>(&'a mut self, outputs: &'a PyList, run_inference: bool) -> PyResult<&'a PyDict> {
        // stage all of the outputs
        let mut values = vec![];
        for output in outputs.into_iter() {
            let val: &PyCell<HopStream> = output.downcast()?;
            if let ProgramState::Building(builder) = &mut self.state {
                values.push(val.borrow_mut().stream.into_list(builder))
            } else {
                return Err(PyRuntimeError::new_err(
                    "Cannot run a program that is not in the Builder state",
                ));
            }
        }

        self.state.initialize(run_inference)?;
        let cycles = self.state.run()?;
        let collected: Vec<_> = values
            .into_iter()
            .map(|vals| (vals)(outputs.py()))
            .collect();
        let result = PyDict::new(outputs.py());
        result.set_item("cycles", cycles)?;
        result.set_item("outputs", collected.into_py(outputs.py()))?;
        Ok(result)
    }

    fn constant<'a>(&mut self, values: &'a PyAny) -> PyResult<&'a PyCell<HopStream>> {
        if let ProgramState::Building(build) = &mut self.state {
            if let Some(stream) = StreamEnum::from_constant(build, values) {
                Ok(PyCell::new(values.py(), HopStream { stream })?)
            } else {
                Err(PyRuntimeError::new_err(
                    "Did not know how to properly parse the input values",
                ))
            }
        } else {
            Err(PyRuntimeError::new_err(
                "Attempted to add a constant to a program that is not in the building state.",
            ))
        }
    }
}
