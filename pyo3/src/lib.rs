mod homomorphism;
mod module;
mod resolution;

use pyo3::prelude::*;
use pyo3::wrap_pymodule;

use crate::homomorphism::PyInit_homomorphism;
use crate::module::PyInit_module;
use crate::resolution::PyInit_resolution;

#[pymodule]
fn ext(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(module))?;
    m.add_wrapped(wrap_pymodule!(resolution))?;
    m.add_wrapped(wrap_pymodule!(homomorphism))?;
    Ok(())
}

#[macro_export]
macro_rules! wrapper_type {
    {
        $vis:vis $outer:ident {
            inner: $inner:ty,
            $($name:ident: $ty:ty,)*
        }
    } => {
        #[pyclass]
        pub struct $outer {
            inner: Option<Arc<$inner>>,
            $(
                $name: Option<Arc<$ty>>,
            )*
        }

        impl $outer {
            #[allow(dead_code)]
            pub fn get(&self) -> PyResult<Arc<$inner>> {
                Ok(Arc::clone(
                    self.inner
                        .as_ref()
                        .ok_or(ReferenceError::py_err("Use of freed object"))?,
                ))
            }

            paste::item! {
                $(
                    #[allow(dead_code)]
                    pub fn [<get_ $name>](&self) -> PyResult<Arc<$ty>> {
                        Ok(Arc::clone(
                                self.$name
                                .as_ref()
                                .ok_or(ReferenceError::py_err("Use of freed object"))?,
                        ))
                    }
                )*
            }

            #[allow(dead_code)]
            pub fn from_inner(inner: Arc<$inner>, $($name: Arc<$ty>,)*) -> Self {
                Self {
                    inner: Some(inner),
                    $(
                        $name: Some($name),
                    )*
                }
            }
        }

        #[pymethods]
        impl $outer {
            fn free(&mut self) {
                self.inner.take();
                $(
                    self.$name.take();
                )*
            }
        }
    };
}
