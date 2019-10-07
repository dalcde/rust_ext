use crate::algebra::{Algebra, AlgebraAny, AdemAlgebra};
use crate::chain_complex::{ChainComplex, AugmentedChainComplex, FiniteAugmentedChainComplex, BoundedChainComplex, ChainMap};
use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace};
use crate::module::homomorphism::{ModuleHomomorphism, BoundedModuleHomomorphism, ZeroHomomorphism, FiniteModuleHomomorphism, FreeModuleHomomorphism};
use crate::module::homomorphism::{TruncatedHomomorphism, TruncatedHomomorphismSource, QuotientHomomorphism, QuotientHomomorphismSource};
use crate::module::{Module, FDModule, FreeModule, BoundedModule, FiniteModule};
use crate::module::{QuotientModule as QM, TruncatedModule as TM};

use bivec::BiVec;

use std::collections::HashSet;
use std::sync::Arc;

const PENALTY_UNIT : i32 = 1 << 15;

pub type Yoneda<CC> = FiniteAugmentedChainComplex<
        FiniteModule,
        FiniteModuleHomomorphism<FiniteModule>,
        FiniteModuleHomomorphism<<<CC as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module>,
        <CC as AugmentedChainComplex>::TargetComplex
    >;

fn rate_operation(algebra : &Arc<AlgebraAny>, op_deg : i32, op_idx : usize) -> i32 {
    let mut pref = 0;
    match &**algebra {
        AlgebraAny::AdemAlgebra(a) => pref += rate_adem_operation(a, op_deg, op_idx),
        _ => ()
    };
    pref
}

fn rate_adem_operation(algebra : &AdemAlgebra, deg : i32, idx: usize) -> i32 {
    if algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
    if elt.ps.len() == 0 {
        return 0;
    }

    let mut pref : i32 = 0;
    for i in elt.ps.iter() {
        pref += i.count_ones() as i32;
    }
    pref += deg - (1 << elt.ps[0].trailing_zeros());
    1 << pref
}

#[allow(dead_code)]
fn operation_drop(algebra : &AdemAlgebra, deg : i32, idx: usize) -> i32 {
    if algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
    if elt.ps.len() == 0 {
        return 0;
    }

    let mut first = elt.ps[0];
    let mut drop = 1;
    while first & 1 == 0 {
        first >>= 1;
        drop *= 2;
    }
    deg - drop
}

fn split_mut_borrow<T> (v : &mut Vec<T>, i : usize, j : usize) -> (&mut T, &mut T) {
    assert!(i < j);
    let (first, second) = v.split_at_mut(j);
    (&mut first[i], &mut second[0])
}

pub fn yoneda_representative_element<TCM, TC, CC>(cc : Arc<CC>, s : u32, t : i32, idx : usize) -> Yoneda<CC>
where TCM : BoundedModule,
      TC : ChainComplex<Module=TCM> + BoundedChainComplex,
      CC : AugmentedChainComplex<TargetComplex=TC, Module=FreeModule, ChainMap=FreeModuleHomomorphism<TCM>> {
    let p = cc.prime();

    let target = FDModule::new(cc.algebra(), "".to_string(), BiVec::from_vec(0, vec![1]));
    let map = FreeModuleHomomorphism::new(cc.module(s), Arc::new(target), t);
    let mut new_output = Matrix::new(p, cc.module(s).number_of_gens_in_degree(t), 1);
    new_output[idx].set_entry(0, 1);

    let lock = map.lock();
    map.add_generators_from_matrix_rows(&lock, t, &mut new_output, 0, 0);
    drop(lock);

    let cm = ChainMap {
        s_shift : s,
        chain_maps : vec![map]
    };
    yoneda_representative(cc, cm)
}

/// This function produces a quasi-isomorphic quotient of `cc` (as an augmented chain complex) that `map` factors through
pub fn yoneda_representative<TCM, TC, CC, CMM>(cc : Arc<CC>, map : ChainMap<FreeModuleHomomorphism<CMM>>) -> Yoneda<CC>
where TCM : BoundedModule,
      TC : ChainComplex<Module=TCM> + BoundedChainComplex,
      CC : AugmentedChainComplex<TargetComplex=TC, Module=FreeModule, ChainMap=FreeModuleHomomorphism<TCM>>,
      CMM : BoundedModule
{
    yoneda_representative_with_strategy(cc, map,
        |module : &FreeModule, subspace : &Subspace, t : i32, i : usize| {
            let opgen = module.index_to_op_gen(t, i);

            let mut pref = rate_operation(&module.algebra(), opgen.operation_degree, opgen.operation_index);

            for k in 0 .. subspace.matrix.rows() {
                if subspace.matrix[k].entry(i) != 0 {
                    pref += PENALTY_UNIT;
                }
            }
            pref
        })
}

pub fn yoneda_representative_with_strategy<TCM, TC, CC, CMM, F>(cc : Arc<CC>, map : ChainMap<FreeModuleHomomorphism<CMM>>, strategy : F) -> Yoneda<CC>
where TCM : BoundedModule,
      TC : ChainComplex<Module=TCM> + BoundedChainComplex,
      CC : AugmentedChainComplex<TargetComplex=TC, Module=FreeModule, ChainMap=FreeModuleHomomorphism<TCM>>,
      CMM : BoundedModule,
      F : Fn(&CC::Module, &Subspace, i32, usize) -> i32 {
    let p = cc.prime();
    let target_cc = cc.target();
    let algebra = cc.algebra();

    let t_shift : i32 = map.chain_maps[0].degree_shift();
    let s_shift : u32 = map.s_shift;

    let s_max = std::cmp::max(target_cc.max_s(), map.s_shift + map.chain_maps.len() as u32) - 1;
    let t_max = std::cmp::max(
        (0 .. target_cc.max_s()).map(|i| target_cc.module(i).max_degree()).max().unwrap_or(target_cc.min_degree()),
        map.chain_maps[0].degree_shift() + map.chain_maps.iter().map(|m| m.target().max_degree()).max().unwrap()
    );

    let t_min = cc.min_degree();

    let mut modules = (0 ..= s_max).map(|s| QM::new(Arc::new(TM::new(cc.module(s), t_max)))).collect::<Vec<_>>();

    for m in &modules {
        m.compute_basis(t_max); // populate masks/basis
    }

    for s in (1 ..= s_max).rev() {
        let (target, source) = split_mut_borrow(&mut modules, s as usize - 1, s as usize);

        for t in (t_min ..= t_max).rev() {
            if t - (s as i32) < cc.min_degree() {
                continue;
            }
            if cc.module(s).dimension(t) == 0 {
                continue;
            }

            let augmentation_map = if s < target_cc.max_s() && target_cc.module(s).dimension(t) > 0 { Some(cc.chain_map(s)) } else { None };
            let preserve_map = if s >= s_shift && t >= t_shift {
                match map.chain_maps.get((s - s_shift) as usize) {
                    Some(m) => if m.target().dimension(t - t_shift) > 0 { Some(m) } else { None },
                    None => None
                }
            } else { None };

            // We can only quotient out by things in the kernel of the augmentation maps *and* the
            // steenrod operations. The function computes the kernel.
            let matrix = compute_kernel(source, augmentation_map, preserve_map, t);

            let subspace = &source.subspaces[t];

            let mut rows = matrix.into_vec();

            rows.sort_by_cached_key(|v|{
                let mut sum : i32 = 0;
                let mut num : i32 = 0;
                for (i, x) in v.iter().enumerate() {
                    if x == 0 {
                        continue;
                    }
                    num += 1;
                    let opgen = source.module.module.index_to_op_gen(t, i);
                    sum += rate_operation(&algebra, opgen.operation_degree, opgen.operation_index);

                    for r in subspace.matrix.iter() {
                        if r.entry(i) != 0 {
                            sum += PENALTY_UNIT;
                        }
                    }
//                    if opgen.generator_degree < t / 2 {
//                        sum += 100 * PENALTY_UNIT;
//                    }
                }
                - sum / num
            });

            let mut dx = FpVector::new(p, target.module.dimension(t));

            let d = cc.differential(s);

            let mut goal_s_dim = source.dimension(t);
            let mut goal_t_dim = target.dimension(t);

            let mut source_kills : Vec<FpVector> = Vec::with_capacity(source.module.dimension(t));

            for row in rows.into_iter() {
                d.apply(&mut dx, 1, t, &row);
                target.subspaces[t].reduce(&mut dx);

                if !dx.is_zero_pure() {
                    source_kills.push(row);
                    target.quotient(t, &dx);
                    dx.set_to_zero_pure();
                } else if s == s_max {
                    source_kills.push(row);
                }
            }
            if s != s_max {
                goal_s_dim -= source_kills.len();
                goal_t_dim -= source_kills.len();
            }

            source.quotient_vectors(t, source_kills);

            if s != s_max {
                assert_eq!(source.dimension(t), goal_s_dim, "Failed s dimension check at (s, t) = ({}, {})", s, t);
                assert_eq!(target.dimension(t), goal_t_dim, "Failed t dimension check at (s, t) = ({}, {})", s, t);
            }

        }
    }

    let zero_module = Arc::new(QM::new(Arc::new(TM::new(cc.zero_module(), t_max))));
    zero_module.compute_basis(t_max);
    let zero_module_fd = Arc::new(FiniteModule::FDModule(zero_module.to_fd_module()));

    let modules_fd = modules.iter().map(|m| Arc::new(FiniteModule::FDModule(m.to_fd_module()))).collect::<Vec<_>>();
    let modules = modules.into_iter().map(Arc::new).collect::<Vec<_>>();

    let zero_differential = {
        let f = cc.differential(0);
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[0].module), Arc::clone(&zero_module.module)));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphism::new(tf, Arc::clone(&modules[0]), Arc::clone(&zero_module)));
        Arc::new(FiniteModuleHomomorphism::from(
            qf.replace_source(Arc::clone(&modules_fd[0]))
              .replace_target(Arc::clone(&zero_module_fd))))
    };

    let mut differentials = vec![zero_differential];
    differentials.extend((0 .. s_max).into_iter().map(|s| {
        let f = cc.differential(s + 1);
        let s = s as usize;
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[s + 1].module), Arc::clone(&modules[s].module)));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphism::new(tf, Arc::clone(&modules[s + 1]), Arc::clone(&modules[s])));
        Arc::new(FiniteModuleHomomorphism::from(
            qf.replace_source(Arc::clone(&modules_fd[s + 1]))
              .replace_target(Arc::clone(&modules_fd[s]))))
    }));
    differentials.push(Arc::new(FiniteModuleHomomorphism::from(BoundedModuleHomomorphism::zero_homomorphism(Arc::clone(&zero_module_fd), Arc::clone(&modules_fd[s_max as usize]), 0))));
    differentials.push(Arc::new(FiniteModuleHomomorphism::from(BoundedModuleHomomorphism::zero_homomorphism(Arc::clone(&zero_module_fd), Arc::clone(&zero_module_fd), 0))));

    let chain_maps = (0 ..= s_max).into_iter().map(|s| {
        let f = cc.chain_map(s);
        let s = s as usize;
        let target = f.target();
        let tf = Arc::new(TruncatedHomomorphismSource::new(f, Arc::clone(&modules[s].module), Arc::clone(&target)));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphismSource::new(tf, Arc::clone(&modules[s]), target));
        Arc::new(FiniteModuleHomomorphism::from(qf.replace_source(Arc::clone(&modules_fd[s]))))
    }).collect::<Vec<_>>();

    FiniteAugmentedChainComplex {
        modules: modules_fd,
        zero_module: zero_module_fd,
        differentials,
        target_cc : cc.target(),
        chain_maps
    }
}

/// This function does the following computation:
///
/// Given the source module `source` and a subspace `keep`, the function returns the subspace of all
/// elements in `source` of degree `t` that are killed by all non-trivial actions of the algebra,
/// followed by a list of elements that span the intersection between this subspace and `keep`.
///
/// If `keep` is `None`, it is interpreted as the empty subspace.

fn compute_kernel<M : BoundedModule, F : ModuleHomomorphism, G : ModuleHomomorphism>(
    source : &QM<M>,
    augmentation_map : Option<Arc<F>>,
    preserve_map : Option<&G>,
    t : i32) -> Matrix {

    let algebra = source.algebra();
    let p = algebra.prime();

    let mut generators : Vec<(i32, usize)> = Vec::new();
    let mut target_degrees = Vec::new();
    let mut padded_target_degrees : Vec<usize> = Vec::new();

    let source_orig_dimension = source.module.dimension(t);

    for op_deg in 1 ..= source.max_degree() - t {
        for op_idx in algebra.generators(op_deg) {
            generators.push((op_deg, op_idx));
            target_degrees.push(source.module.dimension(t + op_deg));
            padded_target_degrees.push(FpVector::padded_dimension(p, source.module.dimension(t + op_deg)));
        }
    }

    if let Some(m) = &augmentation_map {
        target_degrees.push(m.target().dimension(t));
        padded_target_degrees.push(FpVector::padded_dimension(p, m.target().dimension(t)));
    }

    if let Some(m) = &preserve_map {
        let dim = m.target().dimension(t - m.degree_shift());
        target_degrees.push(dim);
        padded_target_degrees.push(FpVector::padded_dimension(p, dim));
    }

    let total_padded_degree : usize = padded_target_degrees.iter().sum();

    let total_cols : usize = total_padded_degree + source_orig_dimension;

    let mut matrix_rows : Vec<FpVector> = Vec::with_capacity(source_orig_dimension);

    for i in 0 .. source_orig_dimension {
        let mut result = FpVector::new(p, total_cols);

        let mut offset = 0;

        let mut target_idx = 0;
        for (op_deg, op_idx) in generators.iter() {
            result.set_slice(offset, offset + target_degrees[target_idx]);
            source.act_on_original_basis(&mut result, 1, *op_deg, *op_idx, t, i);
            result.clear_slice();
            offset += padded_target_degrees[target_idx];
            target_idx += 1;
        }

        if let Some(m) = &augmentation_map {
            result.set_slice(offset, offset + target_degrees[target_idx]);
            m.apply_to_basis_element(&mut result, 1, t, i);
            result.clear_slice();
            offset += padded_target_degrees[target_idx];
            target_idx += 1;
        }

        if let Some(m) = &preserve_map {
            result.set_slice(offset, offset + target_degrees[target_idx]);
            m.apply_to_basis_element(&mut result, 1, t, i);
            result.clear_slice();
        }

        result.set_entry(total_padded_degree + i, 1);
        matrix_rows.push(result);
    }

    let mut matrix = Matrix::from_rows(p, matrix_rows, total_cols);
    let mut pivots = vec![-1; total_cols];
    matrix.row_reduce(&mut pivots);

    let first_kernel_row = match &pivots[0..total_padded_degree].iter().rposition(|&i| i >= 0) {
        Some(n) => pivots[*n] as usize + 1,
        None => 0
    };

    matrix.set_slice(first_kernel_row, source_orig_dimension, total_padded_degree, total_cols);
    matrix.into_slice();

    matrix
}
