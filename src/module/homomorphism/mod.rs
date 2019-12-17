use std::sync::Arc;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace, QuasiInverse};
use crate::module::Module;

mod free_module_homomorphism;
mod generic_zero_homomorphism;
mod bounded_module_homomorphism;
mod hom_pullback;
mod truncated_homomorphism;
mod quotient_homomorphism;
mod fp_module_homomorphism;
mod finite_module_homomorphism;

pub use free_module_homomorphism::FreeModuleHomomorphism;
pub use generic_zero_homomorphism::GenericZeroHomomorphism;
pub use hom_pullback::HomPullback;
pub use bounded_module_homomorphism::BoundedModuleHomomorphism;
pub use finite_module_homomorphism::FiniteModuleHomomorphism;
pub use fp_module_homomorphism::{FPModuleHomomorphism, FPModuleT};
pub use truncated_homomorphism::{TruncatedHomomorphism, TruncatedHomomorphismSource};
pub use quotient_homomorphism::{QuotientHomomorphism, QuotientHomomorphismSource};

pub trait ModuleHomomorphism : Send + Sync + 'static {
    type Source : Module;
    type Target : Module;
    const CUSTOM_QI : bool = false;

    fn source(&self) -> Arc<Self::Source>;
    fn target(&self) -> Arc<Self::Target>;
    fn degree_shift(&self) -> i32;

    /// Calling this function when `input_idx < source().dimension(input_degree)` results in
    /// undefined behaviour. Implementations are encouraged to panic when this happens (this is
    /// usually the case because of out-of-bounds errors.
    fn apply_to_basis_element(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input_idx : usize);

    fn kernel(&self, degree : i32) -> &Subspace;

    fn quasi_inverse(&self, degree : i32) -> &QuasiInverse;

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree : i32);

    fn apply(&self, result : &mut FpVector, coeff : u32, input_degree : i32, input : &FpVector){
        let p = self.prime();
        for (i, v) in input.iter().enumerate() {
            if v==0 { continue; }
            self.apply_to_basis_element(result, (coeff * v) % p, input_degree, i);
        }
    }
    
    fn prime(&self) -> u32 {
        self.source().prime()
    }

    fn min_degree(&self) -> i32 {
        self.source().min_degree()
    }

    /// Returns the image of the module homomorphism in degree `degree`. If `None`, the image
    /// is the whole space.
    fn image(&self, degree : i32) -> &Option<Subspace> {
        &self.quasi_inverse(degree).image
    }

    /// A version of kernel_and_quasi_inverse that, in fact, doesn't compute the kernel.
    fn calculate_quasi_inverse(&self, degree : i32) -> QuasiInverse {
        let p = self.prime();
        self.source().compute_basis(degree);
        self.target().compute_basis(degree);
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(degree);
        let padded_target_dimension = FpVector::padded_dimension(p, target_dimension);
        let columns = padded_target_dimension + source_dimension;
        let mut matrix = Matrix::new(p, source_dimension, columns);

        matrix.set_slice(0, source_dimension, 0, target_dimension);
        self.get_matrix(&mut matrix, degree);
        matrix.clear_slice();

        for i in 0..source_dimension {
            matrix[i].set_entry(padded_target_dimension + i, 1);
        }
        let mut pivots = vec![-1;columns];
        matrix.row_reduce(&mut pivots);
        matrix.compute_quasi_inverse(&pivots, target_dimension, padded_target_dimension)
    }

    fn kernel_and_quasi_inverse(&self, degree : i32) -> (Subspace, QuasiInverse) {
        let p = self.prime();
        self.source().compute_basis(degree);
        self.target().compute_basis(degree);
        let source_dimension = self.source().dimension(degree);
        let target_dimension = self.target().dimension(degree);
        let padded_target_dimension = FpVector::padded_dimension(p, target_dimension);
        let columns = padded_target_dimension + source_dimension;
        let mut matrix = Matrix::new(p, source_dimension, columns);

        matrix.set_slice(0, source_dimension, 0, target_dimension);
        self.get_matrix(&mut matrix, degree);
        matrix.clear_slice();

        for i in 0..source_dimension {
            matrix[i].set_entry(padded_target_dimension + i, 1);
        }
        let mut pivots = vec![-1;columns];
        matrix.row_reduce(&mut pivots);
        let quasi_inverse = matrix.compute_quasi_inverse(&pivots, target_dimension, padded_target_dimension);
        let kernel = matrix.compute_kernel(&pivots, padded_target_dimension);
        (kernel, quasi_inverse)
    }

    /// The (sliced) dimensions of `matrix` must be equal to source_dimension x
    /// target_dimension
    fn get_matrix(&self, matrix : &mut Matrix, degree : i32) {
        if self.target().dimension(degree) == 0 {
            return;
        }

        assert_eq!(self.source().dimension(degree), matrix.rows());
        assert_eq!(self.target().dimension(degree), matrix.columns());

        for (i, row) in matrix.iter_mut().enumerate() {
            self.apply_to_basis_element(row, 1, degree, i);
        }
    }

    fn apply_quasi_inverse(&self, result : &mut FpVector, degree : i32, input : &FpVector) {
        let qi = self.quasi_inverse(degree);
        qi.apply(result, 1, input);
    }
}

pub trait ZeroHomomorphism<S : Module, T : Module> {
    fn zero_homomorphism(s : Arc<S>, t : Arc<T>, degree_shift : i32) -> Self;
}
