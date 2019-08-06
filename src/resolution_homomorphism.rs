use std::rc::Rc;

use crate::once::OnceVec;
use crate::fp_vector::{ FpVector, FpVectorT };
use crate::matrix::Matrix;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::ChainComplex;
use crate::resolution::Resolution;

struct ResolutionHomomorphism<
    S : Module + Sized, F1 : ModuleHomomorphism<S, S> + Sized, CC1 : ChainComplex<S, F1>,
    T : Module + Sized, F2 : ModuleHomomorphism<T, T> + Sized, CC2 : ChainComplex<T, F2>
> {
    source : Rc<Resolution<S, F1, CC1>>,
    target : Rc<Resolution<T, F2, CC2>>,
    maps : OnceVec<FreeModuleHomomorphism<FreeModule>>,
    homological_degree_shift : u32,
    internal_degree_shift : i32
}

impl<
    S : Module + Sized, F1 : ModuleHomomorphism<S, S> + Sized, CC1 : ChainComplex<S, F1>,
    T : Module + Sized, F2 : ModuleHomomorphism<T, T> + Sized, CC2 : ChainComplex<T, F2>
> ResolutionHomomorphism<S, F1, CC1, T, F2, CC2> {
    fn new(
        source : Rc<Resolution<S,F1,CC1>>, target : Rc<Resolution<T,F2,CC2>>,
        homological_degree_shift : u32, internal_degree_shift : i32
    ) -> Self {
        Self {
            source,
            target,
            maps : OnceVec::new(),
            homological_degree_shift,
            internal_degree_shift
        }
    }

    fn get_map(&self, output_homological_degree : u32) -> &FreeModuleHomomorphism<FreeModule>{
        &self.maps[output_homological_degree as usize]
    }


    fn extend(&self, source_homological_degree : u32, source_degree : i32){

    }

    fn extend_step(&self, input_homological_degree : u32, input_internal_degree : i32, extra_images : Option<Matrix>){
        let output_homological_degree = input_homological_degree - self.homological_degree_shift;
        let f_cur = self.get_map(output_homological_degree);
        let num_gens = f_cur.get_source().get_number_of_gens_in_degree(input_internal_degree);
        let mut outputs = self.extend_step_helper(input_homological_degree, input_internal_degree, extra_images);
        let lock = f_cur.get_lock();
        f_cur.add_generators_from_matrix_rows(&lock, input_internal_degree, &mut outputs, 0, 0, num_gens);
    }

    fn extend_step_helper(&self, input_homological_degree : u32, input_internal_degree : i32, extra_images : Option<Matrix>) -> Matrix {
        let p = self.source.get_prime();
        assert!(input_homological_degree >= self.homological_degree_shift);
        let output_homological_degree = input_homological_degree - self.homological_degree_shift;
        let output_internal_degree = input_internal_degree - self.internal_degree_shift;        
        let target_chain_map = self.target.get_chain_map(output_homological_degree);
        let target_chain_map_qi = target_chain_map.get_quasi_inverse(output_internal_degree);
        let target_cc_dimension = target_chain_map.get_target().get_dimension(output_internal_degree);
        if let Some(extra_images_matrix) = &extra_images {
            assert!(target_cc_dimension == extra_images_matrix.get_columns());
        }
        let f_cur = self.get_map(output_homological_degree);
        let num_gens = f_cur.get_source().get_number_of_gens_in_degree(input_internal_degree);
        let fx_dimension = f_cur.get_target().get_dimension(output_internal_degree);
        let mut outputs_matrix = Matrix::new(p, num_gens, fx_dimension);
        if num_gens == 0 || fx_dimension == 0 {
            return outputs_matrix;
        }      
        if output_homological_degree == 0 {
            let extra_images_matrix = extra_images.as_ref().unwrap();
            assert!(num_gens == extra_images_matrix.get_rows());
            for k in 0 .. num_gens {
                let extra_image_matrix = extra_images.as_ref().expect("Missing extra image rows");
                target_chain_map_qi.as_ref().unwrap().apply(&mut outputs_matrix[k], 1, &extra_image_matrix[k])
            }
            return outputs_matrix;
        }
        let d_source = self.source.get_differential(input_homological_degree);
        let d_target = self.target.get_differential(output_homological_degree);        
        let f_prev = self.get_map(output_homological_degree - 1);
        // assert!(d_source.get_source() as *const _ as usize == f_cur.get_source() as *const _ as usize);
        // assert!(d_source.get_target() as *const _ as usize == f_prev.get_source() as *const _ as usize);
        // assert!(d_target.get_source() as *const _ as usize == f_cur.get_target() as *const _ as usize);
        // assert!(d_target.get_target() as *const _ as usize == f_prev.get_target() as *const _ as usize);
        let d_quasi_inverse = d_target.get_quasi_inverse(output_internal_degree - 1).unwrap();
        let dx_dimension = f_prev.get_source().get_dimension(input_internal_degree);
        let fdx_dimension = f_prev.get_target().get_dimension(output_internal_degree);
        let mut dx_vector = FpVector::new(p, dx_dimension, 0);
        let mut fdx_vector = FpVector::new(p, fdx_dimension, 0);
        let mut extra_image_row = 0;
        for k in 0 .. num_gens {
            d_source.apply_to_generator(&mut dx_vector, 1, input_internal_degree, k);
            if dx_vector.is_zero() {
                let extra_image_matrix = extra_images.as_ref().expect("Missing extra image rows");
                target_chain_map_qi.as_ref().unwrap().apply(&mut outputs_matrix[k], 1, &extra_image_matrix[extra_image_row]);
                extra_image_row += 1;
            } else {
                f_prev.apply(&mut fdx_vector, 1, input_internal_degree, &dx_vector);
                d_quasi_inverse.apply(&mut outputs_matrix[k], 1, &fdx_vector);
                dx_vector.set_to_zero();
                fdx_vector.set_to_zero();                    
            }
        }
        let num_extra_image_rows = extra_images.map_or(0, |matrix| matrix.get_rows());
        assert!(extra_image_row == num_extra_image_rows, "Extra image rows");
        return outputs_matrix;
    }

}

// FreeModuleHomomorphism *ResolutionHomomorphism_getMap(ResolutionHomomorphism *f, uint homological_degree);
