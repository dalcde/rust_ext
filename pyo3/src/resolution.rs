use pyo3::exceptions::ReferenceError;
use pyo3::prelude::*;

use crate::module::FiniteModule;
use crate::wrapper_type;
use rust_ext::chain_complex::FiniteChainComplex;
use rust_ext::resolution::Resolution as ResolutionRust;
use rust_ext::CCC;
use std::sync::{Arc, RwLock};

wrapper_type! {
    pub Resolution {
        inner: RwLock<ResolutionRust<CCC>>,
    }
}

#[pymethods]
impl Resolution {
    #[staticmethod]
    pub fn from_module(module: &FiniteModule) -> PyResult<Self> {
        let chain_complex = Arc::new(FiniteChainComplex::ccdz(module.get()?));
        let resolution = Arc::new(RwLock::new(ResolutionRust::new(chain_complex, None, None)));
        Ok(Self::from_inner(resolution))
    }

    fn resolve_through_bidegree(self_: PyRef<Self>, s: u32, t: i32) -> PyResult<PyRef<Self>> {
        self_.get()?.read().unwrap().resolve_through_bidegree(s, t);
        Ok(self_)
    }

    fn resolve_through_degree(self_: PyRef<Self>, degree: i32) -> PyResult<PyRef<Self>> {
        self_.get()?.read().unwrap().resolve_through_degree(degree);
        Ok(self_)
    }

    fn dim(&self, s: u32, t: i32) -> PyResult<usize> {
        let self_ = self.get()?;
        let self_ = self_.read().unwrap();
        self_.resolve_through_bidegree(s, t);
        Ok(self_.module(s).number_of_gens_in_degree(t))
    }

    fn graded_dimension_string(&self) -> PyResult<String> {
        Ok(self.get()?.read().unwrap().graded_dimension_string())
    }
}

#[pymodule]
pub fn resolution(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Resolution>()?;

    Ok(())
}
