use pyo3::exceptions::{ReferenceError, ValueError};
use pyo3::prelude::*;

use crate::module::FiniteModule;
use crate::resolution::Resolution;
use crate::wrapper_type;
use fp::vector::{FpVector, FpVectorT};
use rust_ext::algebra::SteenrodAlgebra as SteenrodAlgebraRust;
use rust_ext::chain_complex::ChainComplex;
use rust_ext::module::homomorphism::{
    BoundedModuleHomomorphism, FiniteModuleHomomorphism as FiniteModuleHomomorphismRust,
};
use rust_ext::module::{FDModule, FiniteModule as FiniteModuleRust};
use rust_ext::resolution_homomorphism::ResolutionHomomorphismToUnit;
use rust_ext::CCC;
use std::sync::Arc;

wrapper_type!(
    FiniteModuleHomomorphism,
    FiniteModuleHomomorphismRust<FiniteModuleRust>
);

#[pymethods]
impl FiniteModuleHomomorphism {
    fn lift(
        &self,
        source: PyRef<Resolution>,
        target: PyRef<Resolution>,
    ) -> PyResult<ResolutionHomomorphism> {
        // TODO: check correct module

        let inner = ResolutionHomomorphismToUnit::from_module_homomorphism(
            "".to_string(),
            Arc::clone(&source.get()?.read().unwrap().inner),
            Arc::clone(&target.get()?.read().unwrap().inner),
            &*self.get()?,
        );
        Ok(ResolutionHomomorphism::from_inner(Arc::new(inner)))
    }
}

#[pyclass]
pub struct FDModuleHomomorphismBuilder {
    inner: BoundedModuleHomomorphism<FiniteModuleRust, FiniteModuleRust>,
}

impl FDModuleHomomorphismBuilder {
    pub fn source(&self) -> &FDModule<SteenrodAlgebraRust> {
        self.inner.source.as_fd_module().unwrap()
    }
}

#[pymethods]
impl FDModuleHomomorphismBuilder {
    #[new]
    fn new(
        source: PyRef<FiniteModule>,
        target: PyRef<FiniteModule>,
        degree_shift: i32,
    ) -> PyResult<Self> {
        if source.get()?.is_fd_module() && target.get()?.is_fd_module() {
            Ok(Self {
                inner: BoundedModuleHomomorphism::new(source.get()?, target.get()?, degree_shift),
            })
        } else {
            Err(ValueError::py_err(format!("Cannot construct FDModuleHomomorphism between {} and {}. Both must be finite dimensional modules", source.get()?.type_(), target.get()?.type_())))
        }
    }

    fn build(&self) -> FiniteModuleHomomorphism {
        FiniteModuleHomomorphism::from_inner(Arc::new(FiniteModuleHomomorphismRust::from(
            self.inner.clone(),
        )))
    }

    fn set(mut self__: PyRefMut<Self>, source: String, target: String) -> PyResult<PyRefMut<Self>> {
        let self_ = &mut *self__;

        let shift = self_.inner.degree_shift;
        let (source_deg, source_idx) =
            self_
                .source()
                .string_to_basis_element(&source)
                .ok_or(ValueError::py_err(format!(
                    "Invalid source element: {}",
                    source
                )))?;

        let vec = &mut self_.inner.matrices[source_deg][source_idx];
        let result = self_.inner.target.as_fd_module().unwrap().parse_element(
            &target,
            source_deg + shift,
            vec,
        );
        if result.is_err() {
            vec.set_to_zero_pure();
            Err(ValueError::py_err(format!(
                "Invalid target element: {}. Value of homomorphism on {} set to zero.",
                target, source
            )))
        } else {
            Ok(self__)
        }
    }
}

wrapper_type!(ResolutionHomomorphism, ResolutionHomomorphismToUnit<CCC>);

#[pymethods]
impl ResolutionHomomorphism {
    fn extend(self_: PyRef<Self>, s: u32, t: i32) -> PyResult<PyRef<Self>> {
        self_.get()?.extend(s, t);
        Ok(self_)
    }

    fn act(&self, s: u32, t: i32, idx: usize) -> PyResult<Vec<u32>> {
        // Check source/target to avoid panics
        let inner = self.get()?;
        inner.extend(s, t);
        let source_s = s - inner.homological_degree_shift;
        let source_t = t - inner.internal_degree_shift;

        if inner.target.weak_count() == 0 {
            return Err(ValueError::py_err(
                "Target of resolution homomorphism is freed",
            ));
        }
        let source = inner.source.upgrade().ok_or(ValueError::py_err(
            "Source of resolution homomorphism is freed",
        ))?;

        let mut result = FpVector::new(
            source.prime(),
            source.module(source_s).number_of_gens_in_degree(source_t),
        );
        inner.act(&mut result, s, t, idx);
        Ok(result.to_vector())
    }
}

#[pymodule]
pub fn homomorphism(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<FiniteModuleHomomorphism>()?;
    m.add_class::<FDModuleHomomorphismBuilder>()?;
    m.add_class::<ResolutionHomomorphism>()?;

    Ok(())
}
