use hop::primitives::elem::Elem;
use pyo3::prelude::*;

#[derive(FromPyObject)]
#[pyo3(transparent)]
struct ElemWrapper_f64i64(#[pyo3(from_py_with = "pyobj_to_elem")] Elem<f64, i64>);

fn pyobj_to_elem(pyobj: &PyAny) -> PyResult<Elem<f64, i64>> {}
