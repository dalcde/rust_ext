
use crate::chain_complex::{ChainComplex, AugmentedChainComplex};
use crate::module::{Module, FreeModule, BoundedModule};
use crate::module::{QuotientModule as QM, TruncatedModule as TM};
use crate::module::{TruncatedHomomorphism, TruncatedHomomorphismSource, QuotientHomomorphism, QuotientHomomorphismSource};
use crate::module_homomorphism::{ModuleHomomorphism, FDModuleHomomorphism};
use crate::algebra::{Algebra, AlgebraAny, AdemAlgebra};

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace};

use std::collections::HashSet;
use std::sync::Arc;

const PENALTY_UNIT : u32 = 100;

fn rate_operation(algebra : &AlgebraAny, op_deg : i32, op_idx : usize) -> u32 {
    match algebra {
        AlgebraAny::AdemAlgebra(a) => rate_adem_operation(a, op_deg, op_idx),
        _ => 1
    }
}

fn rate_adem_operation(algebra : &AdemAlgebra, deg : i32, idx: usize) -> u32{
    if algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
    let mut pref = 0;
    for i in elt.ps.iter() {
        let mut i = *i;
        while i != 0 {
            pref += i & 1;
            i >>= 1;
        }
    }
    pref
}

pub struct YonedaRepresentative<CC : AugmentedChainComplex> {
    modules : Vec<Arc<QM<TM<CC::Module>>>>,
    zero_module : Arc<QM<TM<CC::Module>>>,
    differentials : Vec<Arc<FDModuleHomomorphism<QM<TM<CC::Module>>, QM<TM<CC::Module>>>>>,
    target_cc : Arc<CC::TargetComplex>,
    chain_maps : Vec<Arc<FDModuleHomomorphism<QM<TM<CC::Module>>, <CC::ChainMap as ModuleHomomorphism>::Target>>>
}

impl<CC : AugmentedChainComplex> ChainComplex for YonedaRepresentative<CC> {
    type Module = QM<TM<CC::Module>>;
    type Homomorphism = FDModuleHomomorphism<QM<TM<CC::Module>>, QM<TM<CC::Module>>>;

    fn algebra(&self) -> Arc<AlgebraAny> {
        self.target_cc.algebra()
    }
    fn min_degree(&self) -> i32 {
        self.target_cc.min_degree()
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn module(&self, s : u32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s as usize])
    }

    fn differential(&self, s : u32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    fn compute_through_bidegree(&self, homological_degree : u32, internal_degree : i32) {}

    fn set_homology_basis(&self, homological_degree : u32, internal_degree : i32, homology_basis : Vec<usize>) { unimplemented!() }
    fn homology_basis(&self, homological_degree : u32, internal_degree : i32) -> &Vec<usize> { unimplemented!() }
    fn max_homology_degree(&self, homological_degree : u32) -> i32 { std::i32::MAX }

    fn max_computed_homological_degree(&self) -> u32 { std::u32::MAX }
    fn max_computed_degree(&self) -> i32 { std::i32::MAX }
}

impl<CC : AugmentedChainComplex> AugmentedChainComplex for YonedaRepresentative<CC> {
    type TargetComplex = CC::TargetComplex;
    type ChainMap = FDModuleHomomorphism<QM<TM<CC::Module>>, <<CC as AugmentedChainComplex>::ChainMap as ModuleHomomorphism>::Target>;

    fn target(&self) -> Arc<Self::TargetComplex> {
        Arc::clone(&self.target_cc)
    }

    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s as usize])
    }
}

fn split_mut_borrow<T> (v : &mut Vec<T>, i : usize, j : usize) -> (&mut T, &mut T) {
    assert!(i < j);
    let (first, second) = v.split_at_mut(j);
    (&mut first[i], &mut second[0])
}

pub fn yoneda_representative<CC>(cc : Arc<CC>, s_max : u32, t_max : i32, idx : usize) -> YonedaRepresentative<CC>
where CC : AugmentedChainComplex<Module=FreeModule> {
    assert!(s_max > 0);
    let p = cc.prime();
    let algebra = &*cc.algebra(); // Deref to &AlgebraAny

    let mut modules = (0 ..= s_max).map(|s| QM::new(Arc::new(TM::new(cc.module(s), t_max)))).collect::<Vec<_>>();

    for m in &modules {
        m.compute_basis(t_max); // populate masks/basis
    }

    for t in (0 ..= t_max).rev() {
        let mut keep : Option<Subspace>;
        if t == t_max {
            let mut keep_ = Subspace::new(p, 1, modules[s_max as usize].dimension(t));
            keep_.add_basis_elements(vec![idx].into_iter());
            keep = Some(keep_);
        } else {
            keep = None;
        }

        for s in (0 .. s_max).rev() {
            if t - (s as i32) < cc.min_degree() {
                continue;
            }

            let (target, source) = split_mut_borrow(&mut modules, s as usize, s as usize + 1);

            if source.dimension(t) == 0 {
                keep = None;
                continue;
            }

            let (mut matrix, images) = compute_kernel_image(p, source, keep, t);

            let mut pivots = vec![-1; matrix.columns()];
            matrix.row_reduce(&mut pivots);

            let subspace = &source.subspaces[t];
            let mut pivot_columns : Vec<(u32, usize)> = pivots
                .into_iter()
                .enumerate()
                .filter(|&(i, v)| v >= 0)
                .map(|(i, v)| {
                    let opgen = source.module.module.index_to_op_gen(t, i);

                    let mut pref = rate_operation(algebra, opgen.operation_degree, opgen.operation_index);

                    for k in 0 .. subspace.matrix.rows() {
                        // This means we have quotiented out by something
                        if subspace.matrix[k].entry(i) != 0 {
                            pref += PENALTY_UNIT;
                        }
                    }

                    (pref, i)
                })
                .collect::<Vec<_>>();
            pivot_columns.sort();

            let mut chosen_cols : HashSet<usize> = HashSet::new();

            'outer: for image in images {
                for (_, col) in pivot_columns.iter() {
                    if chosen_cols.contains(col) {
                        continue;
                    }
                    if image.entry(*col) != 0 {
                        chosen_cols.insert(*col);
                        continue 'outer;
                    }
                }
                panic!();
            }

            let mut pivot_columns = pivot_columns.iter().map(|(p, i)| i).collect::<Vec<_>>();
            pivot_columns.sort();

            let d = cc.differential(s + 1);

            let mut matrix = matrix.into_vec();
            let mut source_kills : Vec<FpVector> = Vec::with_capacity(source.module.dimension(t));
            let mut target_kills : Vec<FpVector> = Vec::with_capacity(target.module.dimension(t));

            for col in pivot_columns.into_iter().rev() {
                let source_row = matrix.pop().unwrap();
                if chosen_cols.contains(&col) {
                    continue;
                }

                let mut target_row = FpVector::new(p, target.module.dimension(t));
                d.apply(&mut target_row, 1, t, &source_row);

                source_kills.push(source_row);
                target_kills.push(target_row);
            }
            source.quotient_vectors(t, source_kills);
            target.quotient_vectors(t, target_kills);

            // Finally, record the differentials.
            let source_dim = source.dimension(t);
            let target_dim = target.module.dimension(t);

            let mut differentials = Vec::with_capacity(source_dim);

            for i in 0 .. source_dim {
                let i = source.basis_list[t][i];
                let mut target_kill_vec = FpVector::new(p, target_dim);
                d.apply_to_basis_element(&mut target_kill_vec, 1, t, i);
                target.subspaces[t].reduce(&mut target_kill_vec);
                differentials.push(target_kill_vec);
            }

            let mut keep_ = Subspace::new(p, source_dim, target_dim);
            keep_.add_vectors(differentials.into_iter());
            keep = Some(keep_);
        }
    }

    let zero_module = Arc::new(QM::new(Arc::new(TM::new(cc.zero_module(), t_max))));

    let modules = modules.into_iter().map(Arc::new).collect::<Vec<_>>();

    let zero_differential = {
        let f = cc.differential(0);
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[0].module), Arc::clone(&zero_module.module)));
        Arc::new(FDModuleHomomorphism::from(QuotientHomomorphism::new(tf, Arc::clone(&modules[0]), Arc::clone(&zero_module))))
    };

    let mut differentials = vec![zero_differential];
    differentials.extend((0 .. s_max).into_iter().map(|s| {
        let f = cc.differential(s + 1);
        let s = s as usize;
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[s + 1].module), Arc::clone(&modules[s].module)));
        Arc::new(FDModuleHomomorphism::from(QuotientHomomorphism::new(tf, Arc::clone(&modules[s + 1]), Arc::clone(&modules[s]))))
    }));

    let chain_maps = (0 ..= s_max).into_iter().map(|s| {
        let f = cc.chain_map(s);
        let s = s as usize;
        let target = f.target();
        let tf = Arc::new(TruncatedHomomorphismSource::new(f, Arc::clone(&modules[s].module), Arc::clone(&target)));
        Arc::new(FDModuleHomomorphism::from(QuotientHomomorphismSource::new(tf, Arc::clone(&modules[s]), target)))
    }).collect::<Vec<_>>();

    YonedaRepresentative {
        modules,
        zero_module,
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

fn compute_kernel_image<M : BoundedModule>(
    p : u32,
    source : &QM<M>,
    keep : Option<Subspace>,
    t : i32) -> (Matrix, Vec<FpVector>) {

    let algebra = &*source.algebra();

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

    let total_padded_degree : usize = padded_target_degrees.iter().sum();
    let padded_source_degree : usize = FpVector::padded_dimension(p, source_orig_dimension);
    let total_cols : usize = total_padded_degree + padded_source_degree + source_orig_dimension;

    let mut matrix_rows : Vec<FpVector> = Vec::with_capacity(source.dimension(t));

    let mut projection_off_keep = FpVector::new(p, source_orig_dimension);

    for i in 0 .. source.dimension(t) {
        let mut result = FpVector::new(p, total_cols);

        let i = source.basis_list[t][i];
        let mut offset = 0;

        for (gen_idx, (op_deg, op_idx)) in generators.iter().enumerate() {
            result.set_slice(offset, offset + target_degrees[gen_idx]);
            source.act_on_original_basis(&mut result, 1, *op_deg, *op_idx, t, i);
            result.clear_slice();
            offset += padded_target_degrees[gen_idx];
        }

        if let Some(keep_) = &keep {
            projection_off_keep.set_to_zero();
            projection_off_keep.set_entry(i, 1);
            keep_.reduce(&mut projection_off_keep);
            result.set_slice(offset, offset + source_orig_dimension);
            result.assign(&projection_off_keep);
            result.clear_slice();
        } else {
            result.set_entry(offset + i, 1);
        }

        result.set_entry(padded_source_degree + total_padded_degree + i, 1);
        matrix_rows.push(result);
    }
    let mut matrix = Matrix::from_rows(p, matrix_rows);
    let mut pivots = vec![-1; total_cols];
    matrix.row_reduce(&mut pivots);

    let first_kernel_row = match &pivots[0..total_padded_degree].iter().rposition(|&i| i >= 0) {
        Some(n) => pivots[*n] as usize + 1,
        None => 0
    };
    let first_image_row = match &pivots[total_padded_degree .. total_padded_degree + source_orig_dimension].iter().rposition(|&i| i >= 0) {
        Some(n) => pivots[*n + total_padded_degree] as usize + 1,
        None => first_kernel_row
    };
    let first_empty_row = match &pivots[total_padded_degree + padded_source_degree .. total_cols].iter().rposition(|&i| i >= 0) {
        Some(n) => pivots[*n + total_padded_degree + padded_source_degree] as usize + 1,
        None => first_image_row
    };

    matrix.set_slice(first_kernel_row, first_empty_row, total_padded_degree + padded_source_degree, total_cols);
    matrix.into_slice();

    let first_image_row = first_image_row - first_kernel_row;

    let mut images = Vec::with_capacity(matrix.rows() - first_image_row);
    for i in first_image_row .. matrix.rows() {
        images.push(matrix[i].clone());
    }
    (matrix, images)
}