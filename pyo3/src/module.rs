use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use pyo3::exceptions::{IOError, ReferenceError, RuntimeError, TypeError, ValueError};
use pyo3::prelude::*;

use bivec::BiVec;
use fp::prime::ValidPrime;
use rust_ext::algebra::SteenrodAlgebra as SteenrodAlgebraRust;
//use rust_ext::algebra::{Algebra, MilnorAlgebra};
use rust_ext::algebra::{AdemAlgebra, Algebra};
use rust_ext::module::FiniteModule as FiniteModuleRust;
use rust_ext::module::{BoundedModule, FDModule, Module, SumModule, TensorModule};

use serde_json::{json, Value};

use crate::resolution::Resolution;
use crate::wrapper_type;

type AnyModuleRust = Arc<dyn Module<Algebra = SteenrodAlgebraRust>>;

wrapper_type! {
    pub AnyModule {
        inner: AnyModuleRust,
    }
}

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

    fn as_finite_module(&self, max_degree: i32) -> PyResult<FiniteModule> {
        let result = self.get()?.truncate_to_fd_module(max_degree);
        Ok(FiniteModule::from_inner(Arc::new(FiniteModuleRust::from(
            result,
        ))))
    }
}

wrapper_type! {
    pub FiniteModule {
        inner: FiniteModuleRust,
    }
}

impl FiniteModule {
    fn from_json_inner(mut json: Value) -> PyResult<Self> {
        let algebra = SteenrodAlgebraRust::from_json(&json, "adem".to_string())
            .map_err(|e| ValueError::py_err(format!("Failed to construct algebra: {}", e)))?;
        let algebra = Arc::new(algebra);
        Ok(Self {
            inner: Some(Arc::new(
                FiniteModuleRust::from_json(algebra, &mut json).map_err(|e| {
                    ValueError::py_err(format!("Failed to construct module: {}", e))
                })?,
            )),
        })
    }
}

#[pymethods]
impl FiniteModule {
    #[new]
    fn new(text: String) -> PyResult<Self> {
        let json = serde_json::from_str(&text).or_else(|_| {
            let f = File::open(text)
                .map_err(|e| IOError::py_err(format!("Failed to open file: {}", e)))?;

            serde_json::from_reader(BufReader::new(f))
                .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))
        })?;

        Self::from_json_inner(json)
    }

    #[staticmethod]
    fn from_file(path: String) -> PyResult<Self> {
        let f =
            File::open(path).map_err(|e| IOError::py_err(format!("Failed to open file: {}", e)))?;

        let json = serde_json::from_reader(BufReader::new(f))
            .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
    }

    #[staticmethod]
    fn from_json(json: String) -> PyResult<Self> {
        let json = serde_json::from_str(&json)
            .map_err(|e| ValueError::py_err(format!("Failed to parse json: {}", e)))?;

        Self::from_json_inner(json)
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

    fn dimension(&self, degree: i32) -> PyResult<usize> {
        Ok(self.get()?.dimension(degree))
    }

    fn resolve(&self) -> PyResult<Resolution> {
        Resolution::from_module(self)
    }

    fn element_name(&self, t: i32, idx: usize) -> PyResult<String> {
        Ok(self.get()?.basis_element_to_string(t, idx))
    }

    fn min_degree(&self) -> PyResult<i32> {
        Ok(self.get()?.min_degree())
    }

    fn max_degree(&self) -> PyResult<i32> {
        if let Some(m) = self.get()?.as_fd_module() {
            let mut max = m.max_degree();
            while m.dimension(max) == 0 && max > m.min_degree() {
                max -= 1;
            }
            Ok(max)
        } else {
            Err(TypeError::py_err(format!(
                "Calling max_degree on module that is not finite dimensional module: {}",
                self.get()?.type_()
            )))
        }
    }
}

#[pyclass]
struct FDModuleBuilder {
    algebra: Option<AdemAlgebra>,
    module: Option<FDModule<SteenrodAlgebraRust>>,
    prime: ValidPrime,
    gen_to_idx: HashMap<String, (i32, usize)>,
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    min_degree: i32,
}

impl FDModuleBuilder {
    fn get(&mut self) -> &mut FDModule<SteenrodAlgebraRust> {
        if self.module.is_none() {
            let algebra = self
                .algebra
                .take()
                .unwrap_or_else(|| AdemAlgebra::new(self.prime, *self.prime != 2, false));
            let algebra = Arc::new(SteenrodAlgebraRust::from(algebra));
            self.module = Some(FDModule::new(
                algebra,
                self.name.clone(),
                BiVec::new(self.min_degree),
            ));
        }
        self.module.as_mut().unwrap()
    }
}

// To be honest I am not too certain about the design of this. Should we just keep all the action
// data around and only construct a module when we try to check/build?
#[pymethods]
impl FDModuleBuilder {
    #[new]
    fn new(p: u32, min_degree: Option<i32>) -> PyResult<Self> {
        let prime = match ValidPrime::try_new(p) {
            Some(x) => x,
            None => return Err(ValueError::py_err(format!("Invalid prime: {}", p))),
        };
        Ok(Self {
            algebra: None,
            module: None,
            gen_to_idx: HashMap::new(),
            name: String::new(),
            prime,
            min_degree: min_degree.unwrap_or(0),
        })
    }

    fn build(&mut self) -> PyResult<FiniteModule> {
        self.check()?;
        Ok(FiniteModule::from_inner(Arc::new(FiniteModuleRust::from(
            self.get().clone(),
        ))))
    }

    fn check(&mut self) -> PyResult<bool> {
        let module = self.get();
        for input_degree in (module.min_degree()..=module.max_degree()).rev() {
            for output_degree in input_degree + 1..=module.max_degree() {
                module.extend_actions(input_degree, output_degree);
                module
                    .check_validity(input_degree, output_degree)
                    .map_err(|e| RuntimeError::py_err(e.to_string()))?;
            }
        }
        Ok(true)
    }
    fn prime(&self) -> u32 {
        *self.prime
    }

    fn set_name(mut self_: PyRefMut<Self>, name: String) -> PyRefMut<Self> {
        if let Some(module) = &mut self_.module {
            module.name = name.clone();
        }
        self_.name = name;
        self_
    }

    fn set_min_degree(mut self_: PyRefMut<Self>, min_degree: i32) -> PyResult<PyRefMut<Self>> {
        if self_.module.is_some() {
            Err(RuntimeError::py_err(
                "Cannot change min degree after started building module",
            ))
        } else {
            self_.min_degree = min_degree;
            Ok(self_)
        }
    }

    fn add_generator(
        mut self_: PyRefMut<Self>,
        degree: i32,
        name: String,
    ) -> PyResult<PyRefMut<Self>> {
        if degree < self_.min_degree {
            if self_.module.is_none() {
                self_.min_degree = degree;
            } else {
                return Err(ValueError::py_err(format!(
                    "Degree is {} while minimum degree is {}",
                    degree, self_.min_degree
                )));
            }
        }

        let idx = self_.get().dimension(degree);
        self_.gen_to_idx.insert(name.clone(), (degree, idx));
        self_
            .get()
            .algebra()
            .compute_basis(degree - self_.min_degree);
        self_.get().add_generator(degree, name);
        Ok(self_)
    }

    fn add_action(mut self_: PyRefMut<Self>, action: String) -> PyResult<PyRefMut<Self>> {
        let self__: &mut Self = &mut *self_; // This is needed for the split borrow
        let module = match self__.module.as_mut() {
            Some(x) => x,
            None => return Err(RuntimeError::py_err("Setting action on zero module")),
        };
        module
            .parse_action(&self__.gen_to_idx, &action, true)
            .map_err(|e| ValueError::py_err(e.0))?;
        Ok(self_)
    }
}

#[pymodule]
pub fn module(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<FiniteModule>()?;
    m.add_class::<AnyModule>()?;
    m.add_class::<FDModuleBuilder>()?;

    Ok(())
}
