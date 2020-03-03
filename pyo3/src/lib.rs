use pyo3::exceptions::{ReferenceError, ValueError};
use pyo3::prelude::*;

use rust_ext::algebra::SteenrodAlgebra as SteenrodAlgebraInner;
use rust_ext::module::FiniteModule as FiniteModuleInner;
use rust_ext::module::{BoundedModule, Module, SumModule, TensorModule, TruncatedModule};
use std::sync::Arc;

use serde_json::json;

type AnyModuleInner = Arc<dyn Module<Algebra = SteenrodAlgebraInner>>;

macro_rules! wrapper_type {
    ( $outer:ident, $inner:ident ) => {
        #[pyclass(module = "ext")]
        struct $outer {
            inner: Option<Arc<$inner>>,
        }

        impl $outer {
            #[allow(dead_code)]
            fn get(&self) -> PyResult<Arc<$inner>> {
                Ok(Arc::clone(
                    self.inner
                        .as_ref()
                        .ok_or(ReferenceError::py_err("Use of freed object"))?,
                ))
            }

            #[allow(dead_code)]
            fn from_inner(inner: Arc<$inner>) -> Self {
                Self { inner: Some(inner) }
            }
        }

        #[pymethods]
        impl $outer {
            fn drop(&mut self) {
                self.inner.take();
            }
        }
    };
}

wrapper_type!(AnyModule, AnyModuleInner);

#[pymethods]
impl AnyModule {
    fn dimension(&self, degree: i32) -> PyResult<usize> {
        Ok(self.get()?.dimension(degree))
    }

    fn compute_basis(&self, degree: i32) -> PyResult<()> {
        self.get()?.compute_basis(degree);
        Ok(())
    }

    fn sum(&self, other: &AnyModule) -> PyResult<Self> {
        let other = other.get()?;
        let inner = self.get()?;
        let min_degree = std::cmp::min(inner.min_degree(), other.min_degree());

        Ok(Self {
            inner: Some(Arc::new(Arc::new(SumModule::new(
                inner.algebra(),
                vec![other, inner],
                min_degree,
            )))),
        })
    }

    fn tensor(&self, other: &AnyModule) -> PyResult<Self> {
        Ok(Self {
            inner: Some(Arc::new(Arc::new(TensorModule::new(
                self.get()?,
                other.get()?,
            )))),
        })
    }

    fn to_finite_module(&self, max_degree: i32) -> PyResult<FiniteModule> {
        let truncated = TruncatedModule::new(self.get()?, max_degree);
        let result = truncated.to_fd_module();
        Ok(FiniteModule::from_inner(Arc::new(FiniteModuleInner::from(
            result,
        ))))
    }
}

wrapper_type!(FiniteModule, FiniteModuleInner);

#[pymethods]
impl FiniteModule {
    fn dimension(&self, degree: i32) -> PyResult<usize> {
        Ok(self.get()?.dimension(degree))
    }

    #[staticmethod]
    fn from_json(json: String) -> PyResult<Self> {
        let mut json = serde_json::from_str(&json)
            .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        let algebra = Arc::new(SteenrodAlgebraInner::from_json(&json, "adem".to_string()).unwrap());
        Ok(Self {
            inner: Some(Arc::new(
                FiniteModuleInner::from_json(algebra, &mut json).unwrap(),
            )),
        })
    }

    fn to_json(&self) -> PyResult<String> {
        let mut json = json!({});
        let inner = self.get()?;
        inner.algebra().to_json(&mut json);
        inner.to_json(&mut json);
        Ok(json.to_string())
    }

    fn as_anymodule(&self) -> PyResult<AnyModule> {
        Ok(AnyModule {
            inner: Some(Arc::new(self.get()?)),
        })
    }
}

#[pymodule]
fn ext(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<FiniteModule>()?;
    m.add_class::<AnyModule>()?;

    Ok(())
}
