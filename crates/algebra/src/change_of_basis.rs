use fp::vector::{FpVector, FpVectorT};
use crate::algebra::{Algebra, AdemAlgebra, MilnorAlgebra};
use crate::algebra::adem_algebra::AdemBasisElement;
use crate::algebra::milnor_algebra::MilnorBasisElement;

// use std::sync::Arc;

pub fn adem_to_milnor_on_basis(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, 
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    let elt = adem_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let q = if milnor_algebra.generic { 2 * (*p) - 2 } else { 1 };
    let dim = milnor_algebra.dimension(elt.degree, -1);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }    
    let mut bocksteins = elt.bocksteins;
    let mbe = MilnorBasisElement {
        degree : (q * elt.ps[0] + (bocksteins & 1)) as i32,
        q_part : bocksteins & 1,
        p_part : vec![elt.ps[0]]
    };
    bocksteins >>= 1;
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    let mut total_degree = mbe.degree;
    let cur_dim = milnor_algebra.dimension(total_degree, -1);

    let mut tmp_vector_a = FpVector::new(p, cur_dim);
    let mut tmp_vector_b = FpVector::new(p, 0);
    tmp_vector_a.set_entry(idx, 1);

    for i in 1 .. elt.ps.len() {
        let mbe = MilnorBasisElement {
            degree : (q * elt.ps[i] + (bocksteins & 1)) as i32,
            q_part : bocksteins & 1,
            p_part : vec![elt.ps[i]]
        };
        let idx = milnor_algebra.basis_element_to_index(&mbe);
        bocksteins >>= 1;
        let cur_dim = milnor_algebra.dimension(total_degree + mbe.degree, -1);
        tmp_vector_b.set_scratch_vector_size(cur_dim);
        milnor_algebra.multiply_element_by_basis_element(&mut tmp_vector_b, 1, total_degree, &tmp_vector_a, mbe.degree, idx, -1);
        total_degree += mbe.degree;
        std::mem::swap(&mut tmp_vector_a, &mut tmp_vector_b);
        tmp_vector_b.set_to_zero();
    }
    if bocksteins & 1 == 0 {
        result.add(&tmp_vector_a, coeff);
    } else {
        milnor_algebra.multiply_element_by_basis_element(result, coeff, total_degree, &tmp_vector_a, 1, 0, -1);
    }
}

pub fn adem_to_milnor(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, input : &FpVector
){
    let p = milnor_algebra.prime();
    for (i, v) in input.iter().enumerate() {
        if v == 0 {
            continue;
        }
        adem_to_milnor_on_basis(adem_algebra, milnor_algebra, result, (coeff * v) % *p, degree, i);
    }
}

// This is currently pretty inefficient... We should memoize results so that we don't repeatedly
// recompute the same inverse.
pub fn milnor_to_adem_on_basis(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, 
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    if milnor_algebra.generic {
        milnor_to_adem_on_basis_generic(adem_algebra, milnor_algebra, result, coeff, degree, idx);
    } else {
        milnor_to_adem_on_basis_2(adem_algebra, milnor_algebra, result, coeff, degree, idx);
    }
}

fn milnor_to_adem_on_basis_2(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    let elt = milnor_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let dim = milnor_algebra.dimension(elt.degree, -1);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }
    let mut t = vec![0;elt.p_part.len()];
    t[elt.p_part.len() - 1] = elt.p_part[elt.p_part.len() - 1];
    for i in (0 .. elt.p_part.len() - 1).rev() {
        t[i] = elt.p_part[i] + 2 * t[i + 1];
    }
    let t_idx = adem_algebra.basis_element_to_index(&AdemBasisElement {
        degree,
        excess : 0,
        bocksteins : 0,
        ps : t
    });
    let mut tmp_vector_a = FpVector::new(p, dim);
    adem_to_milnor_on_basis(adem_algebra, milnor_algebra, &mut tmp_vector_a, 1, degree, t_idx);
    assert!(tmp_vector_a.entry(idx) == 1);
    tmp_vector_a.set_entry(idx, 0);
    milnor_to_adem(adem_algebra, milnor_algebra, result, coeff, degree, &tmp_vector_a);
    result.add_basis_element(t_idx, coeff);
}


fn milnor_to_adem_on_basis_generic(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    let elt = milnor_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let dim = milnor_algebra.dimension(elt.degree, -1);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }
    let t_len = std::cmp::max(elt.p_part.len(), (31u32.saturating_sub(elt.q_part.leading_zeros())) as usize);
    let mut t = vec![0;t_len];
    let last_p_part = if t_len <= elt.p_part.len() { elt.p_part[t_len - 1] } else { 0 }; 
    t[t_len - 1] = last_p_part + ((elt.q_part >> (t_len)) & 1);
    for i in (0 .. t_len - 1).rev() {
        let p_part = if i < elt.p_part.len() { elt.p_part[i] } else { 0 };
        t[i] = p_part + ((elt.q_part >> (i + 1)) & 1) + *p * t[i + 1];
    }
    let t_idx = adem_algebra.basis_element_to_index(&AdemBasisElement {
        degree,
        excess : 0,
        bocksteins : elt.q_part,
        ps : t
    });
    let mut tmp_vector_a = FpVector::new(p, dim);
    adem_to_milnor_on_basis(adem_algebra, milnor_algebra, &mut tmp_vector_a, 1, degree, t_idx);
    assert!(tmp_vector_a.entry(idx) == 1);
    tmp_vector_a.set_entry(idx, 0);
    tmp_vector_a.scale(*p - 1);
    milnor_to_adem(adem_algebra, milnor_algebra, result, coeff, degree, &tmp_vector_a);
    result.add_basis_element(t_idx, coeff);
}


pub fn milnor_to_adem(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, input : &FpVector
){
    let p = milnor_algebra.prime();
    for (i, v) in input.iter().enumerate() {
        if v == 0 {
            continue;
        }
        milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, (coeff * v) % *p, degree, i);
    }
}

pub fn adem_q(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, qi : u32
){
    let p = adem_algebra.prime();
    let degree = crate::algebra::combinatorics::tau_degrees(p)[qi as usize];
    let mbe = if adem_algebra.generic {
        MilnorBasisElement {
            degree,
            q_part : 1 << qi, 
            p_part : vec![]
        }
    } else {
        let mut p_part = vec![0; qi as usize + 1];
        p_part[qi as usize] = 1;
        MilnorBasisElement {
            degree,
            q_part: 0,
            p_part
        }
    };
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, coeff, degree, idx);
}

pub fn adem_plist(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, p_part : Vec<u32>
){
    let mbe = MilnorBasisElement {
        degree,
        p_part,
        q_part : 0
    };
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, coeff, degree, idx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use fp::prime::ValidPrime;
    
    #[test]
    fn test_cob_milnor_qs_to_adem(){
        let p = ValidPrime::new(2);
        let max_degree = 16;
        let adem = AdemAlgebra::new(p, *p != 2, false);
        let milnor = MilnorAlgebra::new(p);
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        for (qi, output) in &[
            (0, "P1"),
            (1, "P3 + P2 P1"),
            (2, "P7 + P5 P2 + P6 P1 + P4 P2 P1")
        ] {
            let degree = (1 << (*qi + 1)) - 1;
            let mut result = FpVector::new(p, adem.dimension(degree, -1));
            adem_q(&adem, &milnor, &mut result, 1, *qi);
            println!("Q{} ==> {}", qi, adem.element_to_string(degree, &result));
            assert_eq!(adem.element_to_string(degree, &result), *output)
        }
    }

    #[allow(non_snake_case)]
    #[rstest(p, max_degree,
        case(2, 32),
        case(3, 60)//106 // reduced size of test because we use a slow implementation
    )]    
   fn test_cob_adem_to_milnor(p : u32, max_degree : i32){
        let p = ValidPrime::new(p);
        let adem = AdemAlgebra::new(p, *p != 2, false);
        let milnor = MilnorAlgebra::new(p);//, p != 2
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        
        for degree in 0 .. max_degree {
            println!("degree : {}", degree);
            let dim = adem.dimension(degree, -1);
            let mut milnor_result = FpVector::new(p, dim);
            let mut adem_result = FpVector::new(p, dim);
            for i in 0 .. dim {
                // println!("i : {}", i);
                milnor_to_adem_on_basis(&adem, &milnor, &mut adem_result, 1, degree, i);
                adem_to_milnor(&adem, &milnor, &mut milnor_result, 1, degree, &adem_result);
                assert!(milnor_result.entry(i) == 1, 
                    format!("{} ==> {} ==> {}", 
                        milnor.basis_element_to_string(degree, i),
                        adem.element_to_string(degree, &adem_result),
                        milnor.element_to_string(degree, &milnor_result)
                ));
                milnor_result.set_entry(i, 0);
                assert!(milnor_result.is_zero(),
                    format!("{} ==> {} ==> {}", 
                        milnor.basis_element_to_string(degree, i),
                        adem.element_to_string(degree, &adem_result),
                        milnor.element_to_string(degree, &milnor_result)
                ));
                println!("    {} ==> {}", milnor.basis_element_to_string(degree,i), adem.element_to_string(degree, &adem_result));
                adem_result.set_to_zero();
                milnor_result.set_to_zero();
            }
        }

    }

}
