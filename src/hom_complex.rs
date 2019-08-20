use std::rc::Rc;

use crate::once::{OnceVec, OnceBiVec};
use crate::algebra::AlgebraAny;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::finite_dimensional_module::FiniteDimensionalModuleT;
// use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::{ChainComplex, CochainComplex};
use crate::hom_space::HomSpace;
use crate::hom_pullback::HomPullback;

pub struct HomComplex<CC : ChainComplex<FreeModule, FreeModuleHomomorphism<FreeModule>>, N : FiniteDimensionalModuleT> {
    min_degree : i32,
    source : Rc<CC>,
    target : Rc<N>,
    zero_module : Rc<HomSpace<N>>,
    modules : OnceVec<Rc<HomSpace<N>>>,
    differentials : OnceVec<Rc<HomPullback<N>>>,
    cohomology_basis : OnceVec<OnceBiVec<Vec<usize>>>
}

impl<CC : ChainComplex<FreeModule, FreeModuleHomomorphism<FreeModule>>, N : FiniteDimensionalModuleT>
    HomComplex<CC, N> {
    pub fn new(source : Rc<CC>, target : Rc<N>) -> Self {
        let min_degree = source.min_degree() - target.max_degree();
        let zero_module = Rc::new(HomSpace::new(source.zero_module(), Rc::clone(&target)));
        Self {
            min_degree,
            source,
            target,
            zero_module,
            modules : OnceVec::new(),
            differentials : OnceVec::new(),
            cohomology_basis : OnceVec::new()
        }
    }
}

impl<CC : ChainComplex<FreeModule, FreeModuleHomomorphism<FreeModule>>, N : FiniteDimensionalModuleT>
    CochainComplex<HomSpace<N>, HomPullback<N>> for HomComplex<CC, N> {
    fn algebra(&self) -> Rc<AlgebraAny> {
        self.zero_module.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.min_degree
    }

    fn zero_module(&self) -> Rc<HomSpace<N>> {
        Rc::clone(&self.zero_module)
    }

    fn module(&self, homological_degree : u32) -> Rc<HomSpace<N>> {
        Rc::clone(&self.modules[homological_degree])
    }

    fn differential(&self, homological_degree : u32) -> Rc<HomPullback<N>> {
        Rc::clone(&self.differentials[homological_degree])
    }

    fn set_cohomology_basis(&self, homological_degree : u32, internal_degree : i32, cohomology_basis : Vec<usize>) {
        for i in cohomology_basis.len() ..= homological_degree as usize {
            self.cohomology_basis.push(OnceBiVec::new(self.min_degree()));
        }
        assert!(self.cohomology_basis[homological_degree].len() == internal_degree);
        self.cohomology_basis[homological_degree as usize].push(cohomology_basis);
    }

    fn cohomology_basis(&self, homological_degree : u32, internal_degree : i32) -> &Vec<usize> {
        &self.cohomology_basis[homological_degree as usize][internal_degree]
    }

    fn max_cohomology_degree(&self, homological_degree : u32) -> i32 {
        let homological_degree = homological_degree as usize;
        if homological_degree >= self.cohomology_basis.len(){
            return self.min_degree() - 1;
        }
        return self.cohomology_basis[homological_degree].len();
    }

    fn max_computed_degree(&self) -> i32 {
        let basis : &OnceBiVec<_> = &self.cohomology_basis[0usize];
        return basis.len();
    }

    fn max_computed_homological_degree(&self) -> u32 {
        self.cohomology_basis.len() as u32
    }

    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32){
        self.source.compute_through_bidegree(homological_degree, degree);
        if self.modules.len() == 0 {
            self.modules.push(Rc::new(HomSpace::new(self.source.module(0), Rc::clone(&self.target))));
            self.differentials.push(Rc::new(HomPullback::new(Rc::clone(&self.modules[0u32]), Rc::clone(&self.zero_module), self.source.differential(0))));
        }
        for i in self.modules.len() as u32 ..= homological_degree {
            self.modules.push(Rc::new(HomSpace::new(self.source.module(i), Rc::clone(&self.target))));
            self.differentials.push(Rc::new(HomPullback::new(Rc::clone(&self.modules[i]), Rc::clone(&self.modules[i - 1]), self.source.differential(i))));
        }
    }
}