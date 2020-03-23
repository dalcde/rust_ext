use std::cmp::{min, max};
use std::sync::{Arc, Weak};
use parking_lot::{RwLock, Mutex};
use std::collections::HashSet;

use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};
use fp::matrix::{self, Matrix, Subspace, AugmentedMatrix2, AugmentedMatrix3};
use crate::algebra::Algebra;
use crate::module::{Module, FreeModule};
use once::{OnceVec, OnceBiVec};
use crate::module::homomorphism::{ModuleHomomorphism, FreeModuleHomomorphism};
use crate::chain_complex::{ChainComplex, AugmentedChainComplex, UnitChainComplex, FreeChainComplex};
use crate::resolution_homomorphism::{ResolutionHomomorphism, ResolutionHomomorphismToUnit};

#[cfg(feature = "concurrent")]
use std::{thread, sync::mpsc};
#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

/// ResolutionInner contains the data of the actual resolution, while Resolution contains the bells
/// and whistles such as self maps and callbacks. ResolutionInner is what ResolutionHomomorphism
/// needs to take in, and is always an immutable object, so is wrapped in Arc<> instead of
/// Arc<RwLock>.

/// This separation should make multithreading easier because we only need ResolutionInner to be
/// Send + Sync. In particular, we don't need the callback functions to be Send + Sync.
pub struct ResolutionInner<CC : ChainComplex> {
    complex : Arc<CC>,
    modules : OnceVec<Arc<FreeModule<<CC::Module as Module>::Algebra>>>,
    zero_module : Arc<FreeModule<<CC::Module as Module>::Algebra>>,
    chain_maps : OnceVec<Arc<FreeModuleHomomorphism<CC::Module>>>,
    differentials : OnceVec<Arc<FreeModuleHomomorphism<FreeModule<<CC::Module as Module>::Algebra>>>>,
    kernels : OnceVec<OnceBiVec<Mutex<Option<Subspace>>>>,
    images : OnceVec<OnceBiVec<Mutex<Option<Image>>>>,
    connectivity : i32
}

struct Image {
    matrix : AugmentedMatrix3,
    pivots : Vec<isize>,
    s : u32,
    t : i32
}

// struct Kernel {
//     matrix : AugmentedMatrix2,
//     pivots : Vec<i32>
// }

impl<CC : ChainComplex> ResolutionInner<CC> {
    pub fn new(complex : Arc<CC>) -> Self {
        let algebra = complex.algebra();
        let min_degree = complex.min_degree();
        let zero_module = Arc::new(FreeModule::new(Arc::clone(&algebra), "F_{-1}".to_string(), min_degree));

        Self {
            complex,
            zero_module,

            chain_maps : OnceVec::new(),
            modules : OnceVec::new(),
            differentials : OnceVec::new(),
            kernels : OnceVec::new(),
            images : OnceVec::new(),
            connectivity : 1
        }
    }

    pub fn extended_degree(&self) -> u32 {// (u32, usize) {
        self.modules.len() as u32
    }

    /// This function prepares the ResolutionInner object to perform computations up to the
    /// specified s degree. It does *not* perform any computations by itself. It simply lengthens
    /// the `OnceVec`s `modules`, `chain_maps`, etc. to the right length.
    pub fn extend_through_degree(&self, mut next_s : u32, max_s : u32, next_t : i32, max_t : i32) {
        let min_degree = self.min_degree();

        for i in next_s ..= max_s {
            self.modules.push(Arc::new(FreeModule::new(Arc::clone(&self.algebra()), format!("F{}", i), min_degree + (i as i32) * self.connectivity)));
            self.chain_maps.push(Arc::new(FreeModuleHomomorphism::new(Arc::clone(&self.modules[i]), Arc::clone(&self.complex.module(i)), 0)));
        }

        if next_s == 0 {
            self.kernels.push(OnceBiVec::new(min_degree));
            self.images.push(OnceBiVec::new(min_degree));
            self.kernels[next_s].push(Mutex::new(None));
            self.kernels[next_s].push(Mutex::new(None));
            self.images[next_s].push(Mutex::new(None));
            self.images[next_s].push(Mutex::new(None));
        }

        for s in 0 .. next_s + 1 {
            for _ in next_t + 2 .. max_t {
                self.kernels[s].push(Mutex::new(None));
                self.images[s].push(Mutex::new(None));
            }
        }

        for s in next_s + 1 ..= max_s {
            self.kernels.push(OnceBiVec::new(min_degree));
            self.images.push(OnceBiVec::new(min_degree));
            for _ in min_degree .. max_t + 2 {
                self.kernels[s].push(Mutex::new(None));
                self.images[s].push(Mutex::new(None));
            }
        }

        println!("max_s : {}, max_t : {}", max_s, max_t);
        print!("  kernels: ");
        for k in self.kernels.iter() {
            print!("{}, ", k.len());
        }
        println!("");

        if next_s == 0 {
            self.differentials.push(Arc::new(FreeModuleHomomorphism::new(Arc::clone(&self.modules[0u32]), Arc::clone(&self.zero_module), 0)));
            next_s += 1;
        }
        for i in next_s ..= max_s {
            self.differentials.push(Arc::new(FreeModuleHomomorphism::new(Arc::clone(&self.modules[i]), Arc::clone(&self.modules[i - 1]), 0)));
        }
    }

    /// Call our resolution $X$, and the chain complex to resolve $C$. This is a legitimate
    /// resolution if the map $f: X \to C$ induces an isomorphism on homology. This is the same as
    /// saying the cofiber is exact. The cofiber is given by the complex
    ///
    /// $$ X_s \oplus C_{s+1} \to X_{s-1} \oplus C_s \to X_{s-2} \oplus C_{s-1} \to \cdots $$
    ///
    /// where the differentials are given by
    ///
    /// $$ \begin{pmatrix} d_X & 0 \\\\ (-1)^s f & d_C \end{pmatrix} $$
    ///
    /// Our method of producing $X_{s, t}$ and the chain maps are as follows. Suppose we have already
    /// built the chain map and differential for $X_{s-1, t}$ and $X_{s, t-1}$. Since $X_s$ is a
    /// free module, the generators in degree $< t$ gives us a bunch of elements in $X_s$ already,
    /// and we know exactly where they get mapped to. Let $T$ be the $\\mathbb{F}_p$ vector space
    /// generated by these elements. Then we already have a map
    ///
    /// $$ T \to X_{s-1, t} \oplus C_{s, t}$$
    ///
    /// and we know this hits the kernel of the map
    ///
    /// $$ D = X_{s-1, t} \oplus C_{s, t} \to X_{s-2, t} \oplus C_{s-1, t}. $$
    ///
    /// What we need to do now is to add generators to $X_{s, t}$ to hit the entirity of this
    /// kernel.  Note that we don't *have* to do this. Some of the elements in the kernel might be
    /// hit by $C_{s+1, t}$ and we don't have to hit them, but we opt to add generators to hit it
    /// anyway.
    ///
    /// If we do it this way, then we know the composite of the map
    ///
    /// $$ T \to X_{s-1, t} \oplus C_{s, t} \to C_{s, t} $$
    ///
    /// has to be surjective, since the image of $C_{s, t}$ under $D$ is also in the image of $X_{s-1, t}$.
    /// So our first step is to add generators to $X_{s, t}$ such that this composite is
    /// surjective.
    ///
    /// After adding these generators, we need to decide where to send them to. We know their
    /// values in the $C_{s, t}$ component, but we need to use a quasi-inverse to find the element in
    /// $X_{s-1, t}$ that hits the corresponding image of $C_{s, t}$. This tells us the $X_{s-1,
    /// t}$ component.
    ///
    /// Finally, we need to add further generators to $X_{s, t}$ to hit all the elements in the
    /// kernel of
    ///
    /// $$ X_{s-1, t} \to X_{s-2, t} \oplus C_{s-1, t}. $$
    ///
    /// This kernel was recorded by the previous iteration of the method in `old_kernel`, so this
    /// step is doable as well.
    ///
    /// Note that if we add our new generators conservatively, then the kernel of the maps
    ///
    /// $$
    /// \begin{aligned}
    /// T &\to X_{s-1, t} \oplus C_{s, t} \\\\
    /// X_{s, t} &\to X_{s-1, t} \oplus C_{s, t}
    /// \end{aligned}
    /// $$
    /// agree.
    ///
    /// In the code, we first row reduce the matrix of the map from $T$. This lets us record
    /// the kernel which is what the function returns at the end. This computation helps us perform
    /// the future steps since we need to know about the cokernel of this map.
    ///
    /// # Arguments
    ///  * `s` - The s degree to calculate
    ///  * `t` - The t degree to calculate
    pub fn step_resolution(&self, s : u32, t : i32) {
        println!("s: {}, t: {} || x: {}, y: {}", s, t, t-s as i32, s);
        if s == 0 {
            self.zero_module.extend_by_zero(t);
        }

        let p = self.prime();
        
        //                           current_chain_map
        //                X_{s, t} --------------------> C_{s, t}
        //                   |                               |
        //                   | current_differential          |
        //                   v                               v
        // old_kernel <= X_{s-1, t} -------------------> C_{s-1, t}
        
        let complex = self.complex();
        complex.compute_through_bidegree(s, t + 1);

        let current_differential = self.differential(s);
        let current_chain_map = self.chain_map(s);
        let complex_cur_differential = complex.differential(s);

        match current_differential.next_degree().cmp(&t) {
            std::cmp::Ordering::Greater => {
                // Already computed this degree.
                return;
            }
            std::cmp::Ordering::Less => {
                // Haven't computed far enough yet
                panic!("We're not ready to compute bidegree ({}, {}) yet.", s, t);
            }
            std::cmp::Ordering::Equal => ()
        };

        let source = self.module(s);
        let target_cc = complex.module(s);
        let target_res = current_differential.target(); // This is self.module(s - 1) unless s = 0.
        source.extend_table_entries(t+1);
        target_res.extend_table_entries(t+1);


        let chain_map_lock = current_chain_map.lock();
        let differential_lock = current_differential.lock();
        
        // The Homomorphism matrix has size source_dimension x target_dimension, but we are going to augment it with an
        // identity matrix so that gives a matrix with dimensions source_dimension x (target_dimension + source_dimension).
        // Later we're going to write into this same matrix an isomorphism source/image + new vectors --> kernel
        // This has size target_dimension x (2*target_dimension).
        // This latter matrix may be used to find a preimage of an element under the differential.
        let target_cc_dimension = target_cc.dimension(t);
        let target_res_dimension = target_res.dimension(t);
        let source_dimension = source.dimension(t);
        let rows = target_cc_dimension + target_res_dimension + source_dimension;


        // Calculate how many pivots are missing / gens to add
        let kernel = self.kernels[s][t].lock().take();
        let mut maybe_image = self.images[s][t].lock().take();
        let mut dummy_image;
        let image : &mut Image;
        if let Some(x) = maybe_image.as_mut() {
            image = x;
            println!("image: s = {}, t = {}", image.s, image.t);
            assert_eq!(image.matrix.segment(0,0).columns(), target_cc_dimension);
            // assert_eq!(image.matrix.segment(1,1).columns(), target_res_dimension);
            assert_eq!(image.matrix.segment(2,2).columns(), rows);
        } else {
            dummy_image = Image {
                matrix : AugmentedMatrix3::new(p, rows, &[target_cc_dimension, target_res_dimension, rows]),
                pivots : vec![-1; target_cc_dimension + target_res_dimension + rows ],
                s : s,
                t : t
            };
            dummy_image.matrix.segment(2, 2).set_identity(rows, 0, 0);
            image = &mut dummy_image;
        }
        
        let matrix = AugmentedMatrix3::new(p, rows, &[target_cc_dimension, target_res_dimension, rows]);

        let matrix = &mut image.matrix;
        let pivots = &mut image.pivots;

        // Now add generators to surject onto C_{s, t}.
        // (For now we are just adding the eventual images of the new generators into matrix, we will update
        // X_{s,t} and f later).
        // We record which pivots exactly we added so that we can walk over the added generators in a moment and
        // work out what dX should to to each of them.
        let first_new_row = source_dimension;
        println!("   target_cc_dimension : {}", target_cc_dimension);
        let new_generators = matrix.inner.extend_to_surjection(first_new_row, 0, target_cc_dimension, &pivots);
        let cc_new_gens = new_generators.len();
        let mut res_new_gens = 0;

        let mut middle_rows = Vec::with_capacity(cc_new_gens);
        if s > 0 {
            if cc_new_gens > 0 {
                // Now we need to make sure that we have a chain homomorphism. Each generator x we just added to 
                // X_{s,t} has a nontrivial image f(x) \in C_{s,t}. We need to set d(x) so that f(dX(x)) = dC(f(x)).
                // So we set dX(x) = f^{-1}(dC(f(x)))
                let prev_chain_map = self.chain_map(s - 1);
                let quasi_inverse = prev_chain_map.quasi_inverse(t);

                let dfx_dim = complex_cur_differential.target().dimension(t);
                let mut dfx = FpVector::new(self.prime(), dfx_dim);

                for (i, column) in new_generators.into_iter().enumerate() {
                    complex_cur_differential.apply_to_basis_element(&mut dfx, 1, t, column);
                    quasi_inverse.apply(&mut *matrix.row_segment(first_new_row + i, 1, 1), 1, &dfx);
                    dfx.set_to_zero_pure();

                    // Keep the rows we produced because we have to row reduce to re-compute
                    // the kernel later, but these rows are the images of the generators, so we
                    // still need them.
                    middle_rows.push(matrix[first_new_row + i].clone());
                }
                // Row reduce again since our activity may have changed the image of dX.
                matrix.row_reduce(pivots);
            }
            // println!("matrix.seg(1) : {}", *matrix.segment(1,1));
            // Now we add new generators to hit any cycles in old_kernel that we don't want in our homology.
            res_new_gens = matrix.inner.extend_image(first_new_row + cc_new_gens, matrix.start[1], matrix.end[1], pivots, kernel.as_ref()).len();

            if cc_new_gens > 0 {
                // Now restore the middle rows.
                for (i, row) in middle_rows.into_iter().enumerate() {
                    matrix[first_new_row + i] = row;
                }
            }
        }

        println!("cc_new_gens : {}, res_new_gens: {}", cc_new_gens, res_new_gens);
        let num_new_gens = cc_new_gens + res_new_gens;
        source.add_generators(t, num_new_gens, None);

        let rows = matrix.rows();
        matrix.set_row_slice(first_new_row, rows);
        current_chain_map.add_generators_from_matrix_rows(&chain_map_lock, t, &*matrix.segment(0, 0));
        current_differential.add_generators_from_matrix_rows(&differential_lock, t, &*matrix.segment(1, 1));
        matrix.clear_row_slice();

        // Record the quasi-inverses for future use.
        // The part of the matrix that contains interesting information is occupied_rows x (target_dimension + source_dimension + kernel_size).
        let image_rows = first_new_row + num_new_gens;
        for i in first_new_row .. image_rows {
            matrix.inner[i].set_entry(matrix.start[2] + i, 1);
        }

        // From now on we only use the underlying matrix. We manipulate slice directly but don't
        // drop matrix so that we can use matrix.start
        matrix.inner.set_slice(0, image_rows, 0, matrix.start[2] + source_dimension + num_new_gens);
        let mut new_pivots = vec![-1;matrix.columns()];
        matrix.row_reduce(&mut new_pivots);

        // Should this be a method on AugmentedMatrix3?
        let (cm_qi, res_qi) = matrix.compute_quasi_inverses(&new_pivots);

        current_chain_map.set_quasi_inverse(&chain_map_lock, t, cm_qi);
        current_chain_map.set_kernel(&chain_map_lock, t, Subspace::new(p, 0, 0)); // Fill it up with something dummy so that compute_kernels_and... is happy
        current_differential.set_quasi_inverse(&differential_lock, t, res_qi);
        current_differential.set_kernel(&differential_lock, t, Subspace::new(p, 0, 0));

        let target_cc_dimension = target_cc.dimension(t+1);
        let target_res_dimension = target_res.dimension(t+1);
        let source_dimension = source.dimension(t+1);
        target_res.extend_table_entries(t+1);
        source.extend_table_entries(t+1);


        // Now we are going to investigate the homomorphism in degree t + 1.

        // Now need to calculate new_kernel and new_image.

        let rows = source_dimension + target_cc_dimension + target_res_dimension;
        let mut matrix = AugmentedMatrix3::new(p, rows, &[target_cc_dimension, target_res_dimension, rows]);
        let mut pivots = vec![-1;matrix.columns()];
        // Get the map (d, f) : X_{s, t} -> X_{s-1, t} (+) C_{s, t} into matrix

        matrix.set_row_slice(0, source_dimension);
        current_chain_map.get_matrix(&mut *matrix.segment(0,0), t + 1);
        current_differential.get_matrix(&mut *matrix.segment(1,1), t + 1);
        matrix.segment(2,2).set_identity(rows, 0, 0);

        matrix.row_reduce(&mut pivots);
        let new_kernel = matrix.inner.compute_kernel(&pivots, matrix.start[2]);
        
        let mut kernel_lock = self.kernels[s][t+1].lock();
        *kernel_lock = Some(new_kernel);
        if s > 0 {
            let mut image_lock = self.images[s - 1][t + 1].lock();
            *image_lock = Some(Image {
                matrix : matrix,
                pivots : pivots,
                s : s - 1,
                t : t + 1
            });
            println!("Storing image into (s: {}, t: {})", s - 1, t + 1);
            drop(image_lock);
        }
        drop(kernel_lock);
        
    }

    pub fn cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> String {
        let p = self.prime();
        let d = self.differential(hom_deg);
        let source = self.module(hom_deg);
        let target = d.target();
        let dimension = target.dimension(int_deg);
        let basis_idx = source.operation_generator_to_index(0, 0, int_deg, idx);
        let mut result_vector = fp::vector::FpVector::new(p, dimension);
        d.apply_to_basis_element(&mut result_vector, 1, int_deg, basis_idx);

        target.element_to_string(int_deg, &result_vector)
    }

    pub fn complex(&self) -> Arc<CC> {
        Arc::clone(&self.complex)
    }

    pub fn number_of_gens_in_bidegree(&self, homological_degree : u32, internal_degree : i32) -> usize {
        self.module(homological_degree).number_of_gens_in_degree(internal_degree)
    }

    pub fn prime(&self) -> ValidPrime {
        self.complex.prime()
    }
}

impl<CC : ChainComplex> ChainComplex for ResolutionInner<CC> {
    type Algebra = CC::Algebra;
    type Module = FreeModule<Self::Algebra>;
    type Homomorphism = FreeModuleHomomorphism<FreeModule<Self::Algebra>>;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.complex().algebra()
    }

    fn module(&self, homological_degree : u32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[homological_degree as usize])
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn min_degree(&self) -> i32 {
        self.complex().min_degree()
    }

    fn set_homology_basis(&self, _homological_degree : u32, _internal_degree : i32, _homology_basis : Vec<usize>){
        unimplemented!()
    }

    fn homology_basis(&self, _homological_degree : u32, _internal_degree : i32) -> &Vec<usize>{
        unimplemented!()
    }

    fn homology_dimension(&self, homological_degree : u32, internal_degree : i32) -> usize {
        self.number_of_gens_in_bidegree(homological_degree, internal_degree)
    }

    fn max_homology_degree(&self, _homological_degree : u32) -> i32 {
        unimplemented!()
    }

    fn differential(&self, s : u32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    fn compute_through_bidegree(&self, s : u32, t : i32) {
        assert!(self.modules.len() > s as usize);
        assert!(self.modules[0 as usize].max_computed_degree() >= t);
    }
}

impl<CC : ChainComplex> AugmentedChainComplex for ResolutionInner<CC> {
    type TargetComplex = CC;
    type ChainMap = FreeModuleHomomorphism<CC::Module>;

    fn target(&self) -> Arc<Self::TargetComplex> {
        self.complex()
    }

    fn chain_map(&self, s : u32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s])
    }
}

/// Hack to compare two pointers of different types (in this case because they might have different
/// type parameters.
fn ptr_eq<T, S>(a : &Arc<T>, b : &Arc<S>) -> bool {
    let a = Arc::into_raw(Arc::clone(a));
    let b = Arc::into_raw(Arc::clone(b)) as *const T;
    let eq = std::ptr::eq(a, b);
    unsafe {
        let _ = Arc::from_raw(a);
        let _ = Arc::from_raw(b as *const S);
    }
    eq
}

#[derive(Clone)]
struct Cocycle {
    s : u32,
    t : i32,
    class : Vec<u32>,
    name : String
}

pub struct SelfMap<CC : ChainComplex> {
    pub s : u32,
    pub t : i32,
    pub name : String,
    pub map_data : Matrix,
    pub map : ResolutionHomomorphism<ResolutionInner<CC>, ResolutionInner<CC>>
}

pub type AddClassFn = Box<dyn Fn(u32, i32, usize)>;
pub type AddStructlineFn = Box<dyn Fn(
    &str,
    u32, i32,
    u32, i32,
    bool,
    Vec<Vec<u32>>
    )>;
    
/// # Fields
///  * `kernels` - For each *internal* degree, store the kernel of the most recently calculated
///  chain map as returned by `generate_old_kernel_and_compute_new_kernel`, to be used if we run
///  resolve_through_degree again.
pub struct Resolution<CC : UnitChainComplex> {
    pub inner : Arc<ResolutionInner<CC>>,

    next_s : Mutex<u32>,
    next_t : Mutex<i32>,
    pub add_class : Option<AddClassFn>,
    pub add_structline : Option<AddStructlineFn>,

    filtration_one_products : Vec<(String, i32, usize)>,

    // Products
    pub unit_resolution : Option<Weak<RwLock<Resolution<CC>>>>,
    pub unit_resolution_owner : Option<Arc<RwLock<Resolution<CC>>>>,
    product_names : HashSet<String>,
    product_list : Vec<Cocycle>,
    // s -> t -> idx -> resolution homomorphism to unit resolution. We don't populate this
    // until we actually have a unit resolution, of course.
    chain_maps_to_unit_resolution : OnceVec<OnceBiVec<OnceVec<ResolutionHomomorphismToUnit<CC>>>>,
    max_product_homological_degree : u32,

    // Self maps
    pub self_maps : Vec<SelfMap<CC>>
}

impl<CC : UnitChainComplex> Resolution<CC> {
    pub fn new(
        complex : Arc<CC>,
        add_class : Option<AddClassFn>,
        add_structline : Option<AddStructlineFn>
    ) -> Self {
        let inner = ResolutionInner::new(complex);
        Self::new_with_inner(inner, add_class, add_structline)
    }

    pub fn new_with_inner(
        inner : ResolutionInner<CC>,
        add_class : Option<AddClassFn>,
        add_structline : Option<AddStructlineFn>
    ) -> Self {
        let inner = Arc::new(inner);
        let min_degree = inner.min_degree();
        let algebra = inner.complex().algebra();

        Self {
            inner,

            next_s : Mutex::new(0),
            next_t : Mutex::new(min_degree),
            add_class,
            add_structline,

            filtration_one_products : algebra.default_filtration_one_products(),

            chain_maps_to_unit_resolution : OnceVec::new(),
            max_product_homological_degree : 0,
            product_names : HashSet::new(),
            product_list : Vec::new(),
            unit_resolution : None,
            unit_resolution_owner : None,

            self_maps : Vec::new()
        }
    }



    #[cfg(feature = "concurrent")]
    pub fn resolve_through_bidegree_concurrent(&self, mut max_s : u32, mut max_t : i32, bucket : &Arc<TokenBucket>) {
        let min_degree = self.min_degree();
        let mut next_s = self.next_s.lock();
        let mut next_t = self.next_t.lock();

        // We want the computed area to always be a rectangle.
        max_t = max(max_t, *next_t - 1);
        if max_s < *next_s {
            max_s = *next_s - 1;
        }

        self.inner.complex().compute_through_bidegree(max_s, max_t);
        self.inner.extend_through_degree(*next_s, max_s, *next_t, max_t);
        self.algebra().compute_basis(max_t - min_degree);

        if let Some(unit_res) = &self.unit_resolution {
            let unit_res = unit_res.upgrade().unwrap();
            let unit_res = unit_res.read();
            // Avoid a deadlock
            if !ptr_eq(&unit_res.inner, &self.inner) {
                unit_res.resolve_through_bidegree_concurrent(self.max_product_homological_degree, max_t - min_degree, bucket);
            }
        }

        let (pp_sender, pp_receiver) = mpsc::channel();
        let mut last_receiver : Option<mpsc::Receiver<()>> = None;
        for t in min_degree ..= max_t {
            let next_t = *next_t;
            let next_s = *next_s;

            let start = if t < next_t { next_s } else { 0 };

            let (sender, receiver) = mpsc::channel();

            let bucket = Arc::clone(bucket);
            let inner = Arc::clone(&self.inner);

            let pp_sender = pp_sender.clone();
            thread::spawn(move || {
                if t == next_t - 1 {
                    for _ in 0 .. next_s {
                        sender.send(()).unwrap();
                    }
                }

                let mut token = bucket.take_token();
                for s in start ..= max_s {
                    token = bucket.recv_or_release(token, &last_receiver);
                    inner.step_resolution(s, t);

                    pp_sender.send((s, t)).unwrap();
                    sender.send(()).unwrap();
                }
            });
            last_receiver = Some(receiver);
        }
        // We drop this pp_sender, so that when all previous threads end, no pp_sender's are
        // present, so pp_receiver terminates.
        drop(pp_sender);

        for (s, t) in pp_receiver {
            self.step_after(s, t);
        }
        *next_s = max_s + 1;
        *next_t = max_t + 1;
    }

    pub fn resolve_through_bidegree(&self, mut max_s : u32, mut max_t : i32) {
        let min_degree = self.min_degree();
        let mut next_s = self.next_s.lock();
        let mut next_t = self.next_t.lock();

        // We want the computed area to always be a rectangle.
        max_t = max(max_t, *next_t - 1);
        if max_s < *next_s {
            max_s = *next_s - 1;
        }

        self.inner.complex().compute_through_bidegree(max_s, max_t);
        self.inner.extend_through_degree(*next_s, max_s, *next_t, max_t);
        self.algebra().compute_basis(max_t - min_degree);

        if let Some(unit_res) = &self.unit_resolution {
            let unit_res = unit_res.upgrade().unwrap();
            let unit_res = unit_res.read();
            // Avoid a deadlock
            if !ptr_eq(&unit_res.inner, &self.inner) {
                unit_res.resolve_through_bidegree(self.max_product_homological_degree, max_t - min_degree);
            }
        }

        for t in min_degree ..= max_t {
            let start = if t < *next_t { *next_s } else { 0 };
            for s in start ..= max_s {
                self.inner.step_resolution(s, t);
                self.step_after(s, t);
            }
        }
        *next_s = max_s + 1;
        *next_t = max_t + 1;
    }

    #[cfg(feature = "concurrent")]
    pub fn resolve_through_degree_concurrent(&self, degree : i32, bucket : &Arc<TokenBucket>) {
        self.resolve_through_bidegree_concurrent(degree as u32, degree, bucket);
    }

    pub fn resolve_through_degree(&self, degree : i32) {
        self.resolve_through_bidegree(degree as u32, degree);
    }

    fn step_after(&self, s : u32, t : i32) {
        if t - (s as i32) < self.min_degree() {
            return;
        }
        let module = self.module(s);
        let num_gens = module.number_of_gens_in_degree(t);
        if let Some(f) = &self.add_class {
            f(s, t, num_gens);
        }
        self.compute_filtration_one_products(s, t);
        self.construct_maps_to_unit(s, t);
        self.extend_maps_to_unit(s, t);
        self.compute_products(s, t, &self.product_list);
        self.compute_self_maps(s, t);
    }

    #[allow(clippy::needless_range_loop)]
    fn compute_filtration_one_products(&self, target_s : u32, target_t : i32){
        if target_s == 0 {
            return;
        }
        let source_s = target_s - 1;

        let source = self.module(source_s);
        let target = self.module(target_s);

        let target_dim = target.number_of_gens_in_degree(target_t);

        for (op_name, op_degree, op_index) in &self.filtration_one_products {
            let source_t = target_t - *op_degree;
            if source_t - (source_s as i32) < self.min_degree(){
                continue;
            }
            let source_dim = source.number_of_gens_in_degree(source_t);

            let d = self.differential(target_s);

            let mut products = vec![Vec::with_capacity(target_dim); source_dim];

            for i in 0 .. target_dim {
                let dx = d.output(target_t, i);

                for j in 0 .. source_dim {
                    let idx = source.operation_generator_to_index(*op_degree, *op_index, source_t, j);
                    products[j].push(dx.entry(idx));
                }
            }

            self.add_structline(op_name, source_s, source_t, target_s, target_t, true, products);
        }
    }

    pub fn add_structline(
            &self,
            name : &str,
            source_s : u32, source_t : i32,
            target_s : u32, target_t : i32,
            left : bool,
            products : Vec<Vec<u32>>
    ){
        if let Some(add_structline) = &self.add_structline {
            add_structline(name, source_s, source_t, target_s, target_t, left, products);
        }
    }

    fn max_computed_degree(&self) -> i32 {
        *self.next_t.lock() - 1
    }

    fn max_computed_homological_degree(&self) -> u32 {
        *self.next_s.lock() - 1
    }

    pub fn graded_dimension_vec(&self) -> Vec<Vec<usize>> {
        let min_degree = self.min_degree();
        let max_degree = self.max_computed_degree();
        let max_hom_deg = self.max_computed_homological_degree();
        let mut result = Vec::with_capacity(max_hom_deg as usize + 1);
        for i in (0 ..= max_hom_deg).rev() {
            let module = self.module(i);
            result.push(
                (min_degree + i as i32 ..= max_degree)
                    .map(|j| module.number_of_gens_in_degree(j))
                    .collect::<Vec<_>>()
            );
        }
        result
    }

    pub fn graded_dimension_string(&self) -> String {
        self.inner.graded_dimension_string(self.max_computed_degree(), self.max_computed_homological_degree())
    }
}

// Product algorithms
impl<CC: UnitChainComplex> Resolution<CC> {
    /// This function computes the products between the element most recently added to product_list
    /// and the parts of Ext that have already been computed. This function should be called right
    /// after `add_product`, unless `resolve_through_degree`/`resolve_through_bidegree` has never been
    /// called.
    ///
    /// This is made separate from `add_product` because extend_maps_to_unit needs a borrow of
    /// `self`, but `add_product` takes in a mutable borrow.
    pub fn catch_up_products(&self) {
        let new_product = [self.product_list.last().unwrap().clone()];
        let next_s = *self.next_s.lock();
        if next_s > 0 {
            let min_degree = self.min_degree();
            let max_s = next_s - 1;
            let max_t = *self.next_t.lock() - 1;

            self.construct_maps_to_unit(max_s, max_t);

            self.extend_maps_to_unit(max_s, max_t);

            for t in min_degree ..= max_t {
                for s in 0 ..= max_s {
                    self.compute_products(s, t, &new_product);
                }
            }
        }
    }

    /// The return value is whether the product was actually added. If the product is already
    /// present, we do nothing.
    pub fn add_product(&mut self, s : u32, t : i32, class : Vec<u32>, name : &str) -> bool {
        let name = name.to_string();
        if self.product_names.contains(&name) {
            false
        } else {
            self.product_names.insert(name.clone());
            self.construct_unit_resolution();
            if s > self.max_product_homological_degree {
                self.max_product_homological_degree = s;
            }

            // We must add a product into product_list before calling compute_products, since
            // compute_products aborts when product_list is empty.
            self.product_list.push(Cocycle { s, t, class, name });
            true
        }
    }

    pub fn construct_unit_resolution(&mut self) {
        if self.unit_resolution.is_none() {
            let ccdz = Arc::new(CC::unit_chain_complex(self.algebra()));
            let unit_resolution = Arc::new(RwLock::new(Resolution::new(ccdz, None, None)));
            self.unit_resolution = Some(Arc::downgrade(&unit_resolution));
            self.unit_resolution_owner = Some(unit_resolution);
        }
    }

    pub fn set_unit_resolution(&mut self, unit_res : Weak<RwLock<Resolution<CC>>>) {
        if !self.chain_maps_to_unit_resolution.is_empty() {
            panic!("Cannot change unit resolution after you start computing products");
        }
        self.unit_resolution = Some(unit_res);
    }

    /// Compute products whose result lie in degrees up to (s, t)
    fn compute_products(&self, s : u32, t : i32, products: &[Cocycle]) {
        for elt in products {
            self.compute_product_step(elt, s, t);
        }
    }

    /// Target = result of the product
    /// Source = multiplicand
    fn compute_product_step(&self, elt : &Cocycle, target_s : u32, target_t : i32) {
        if target_s < elt.s {
            return;
        }
        let source_s = target_s - elt.s;
        let source_t = target_t - elt.t;

        if source_t - (source_s as i32) < self.min_degree() {
            return;
        }

        let source_dim = self.inner.number_of_gens_in_bidegree(source_s, source_t);
        let target_dim = self.inner.number_of_gens_in_bidegree(target_s, target_t);

        let mut products = Vec::with_capacity(source_dim);
        for k in 0 .. source_dim {
            products.push(Vec::with_capacity(target_dim));

            let f = &self.chain_maps_to_unit_resolution[source_s][source_t][k];

            let unit_res_ = self.unit_resolution.as_ref().unwrap().upgrade().unwrap();
            let unit_res = unit_res_.read();
            let output_module = unit_res.module(elt.s);

            for l in 0 .. target_dim {
                let result = f.get_map(elt.s).output(target_t, l);
                let mut val = 0;
                for i in 0 .. elt.class.len() {
                    if elt.class[i] != 0 {
                        let idx = output_module.operation_generator_to_index(0, 0, elt.t, i);
                        val += elt.class[i] * result.entry(idx);
                    }
                }
                products[k].push(val % *self.prime());
            }
        }
        self.add_structline(&elt.name, source_s, source_t, target_s, target_t, true, products);
    }

    fn construct_maps_to_unit(&self, s : u32, t : i32) {
        // If there are no products, we return
        if self.product_list.is_empty() {
            return;
        }

        let p = self.prime();
        let s_idx = s as usize;

        // Populate the arrays if the ResolutionHomomorphisms have not been defined.
        for new_s in 0 ..= s_idx {
            if new_s == self.chain_maps_to_unit_resolution.len() {
                self.chain_maps_to_unit_resolution.push(OnceBiVec::new(self.min_degree()));
            }

            while t >= self.chain_maps_to_unit_resolution[new_s].len() {
                let new_t = self.chain_maps_to_unit_resolution[new_s].len();
                self.chain_maps_to_unit_resolution[new_s].push(OnceVec::new());

                let num_gens = self.module(new_s as u32).number_of_gens_in_degree(new_t);
                if num_gens > 0 {
                    let mut unit_vector = Matrix::new(p, num_gens, 1);
                    for j in 0 .. num_gens {
                        let f = ResolutionHomomorphism::new(
                            format!("(hom_deg : {}, int_deg : {}, idx : {})", new_s, new_t, j),
                            Arc::downgrade(&self.inner), Arc::downgrade(&self.unit_resolution.as_ref().unwrap().upgrade().unwrap().read().inner),
                            new_s as u32, new_t
                            );
                        unit_vector[j].set_entry(0, 1);
                        f.extend_step(new_s as u32, new_t, Some(&unit_vector));
                        unit_vector[j].set_to_zero_pure();
                        self.chain_maps_to_unit_resolution[new_s][new_t].push(f);
                    }
                }
            }
        }
    }

    /// This ensures the chain_maps_to_unit_resolution are defined such that we can compute products up
    /// to bidegree (s, t)
    fn extend_maps_to_unit(&self, s : u32, t : i32) {
        // If there are no products, we return
        if self.product_list.is_empty() {
            return;
        }

        // Now we actually extend the maps.
        let min_degree = self.min_degree();
        for i in 0 ..= s {
            for j in min_degree ..= t {
                let max_s = min(s, i + self.max_product_homological_degree);
                let num_gens = self.module(i).number_of_gens_in_degree(j);
                for k in 0 .. num_gens {
                    let f = &self.chain_maps_to_unit_resolution[i as usize][j][k];
                    f.extend(max_s, t);
                }
            }
        }
    }
}

// Self map algorithms
impl<CC : UnitChainComplex> Resolution<CC> {
    /// The return value is whether the self map was actually added. If the self map is already
    /// present, we do nothing.
    pub fn add_self_map(&mut self, s : u32, t : i32, name : &str, map_data : Matrix) -> bool {
        let name = name.to_string();
        if self.product_names.contains(&name) {
            false
        } else {
            self.product_names.insert(name.clone());
            self.self_maps.push(
                SelfMap {
                    s, t, name, map_data,
                    map : ResolutionHomomorphism::new("".to_string(), Arc::downgrade(&self.inner), Arc::downgrade(&self.inner), s, t)
                });
            true
        }
    }

    /// We compute the products by self maps where the result has degree (s, t).
    #[allow(clippy::needless_range_loop)]
    fn compute_self_maps(&self, target_s : u32, target_t : i32) {
        for f in &self.self_maps {
            if target_s < f.s {
                return;
            }
            let source_s = target_s - f.s;
            let source_t = target_t - f.t;

            if source_t - (source_s as i32) < self.min_degree() {
                continue;
            }
            if source_s == 0 && source_t == self.min_degree() {
                f.map.extend_step(target_s, target_t, Some(&f.map_data));
            }
            f.map.extend(target_s, target_t);

            let source = self.module(source_s);
            let target = self.module(target_s);

            let source_dim = source.number_of_gens_in_degree(source_t);
            let target_dim = target.number_of_gens_in_degree(target_t);

            let mut products = vec![Vec::with_capacity(target_dim); source_dim];

            for j in 0 .. target_dim {
                let result = f.map.get_map(source_s).output(target_t, j);

                for k in 0 .. source_dim {
                    let vector_idx = source.operation_generator_to_index(0, 0, source_t, k);
                    products[k].push(result.entry(vector_idx));
                }
            }
            self.add_structline(&f.name, source_s, source_t, target_s, target_t, false, products);
        }
    }
}

impl<CC : UnitChainComplex> Resolution<CC>
{
    pub fn algebra(&self) -> Arc<<CC::Module as Module>::Algebra> {
        self.inner.complex().algebra()
    }

    pub fn prime(&self) -> ValidPrime {
        self.inner.prime()
    }

    pub fn module(&self, homological_degree : u32) -> Arc<FreeModule<<CC::Module as Module>::Algebra>> {
        self.inner.module(homological_degree)
    }

    pub fn min_degree(&self) -> i32 {
        self.inner.complex().min_degree()
    }

    pub fn differential(&self, s : u32) -> Arc<FreeModuleHomomorphism<FreeModule<<CC::Module as Module>::Algebra>>> {
        self.inner.differential(s)
    }
}

use std::io;
use std::io::{Read, Write};
use saveload::{Save, Load};

impl<CC : ChainComplex> Save for ResolutionInner<CC> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.modules.save(buffer)?;
        // self.kernels.save(buffer)?;
        self.differentials.save(buffer)?;
        self.chain_maps.save(buffer)?;
        Ok(())
    }
}

impl<CC : ChainComplex> Load for ResolutionInner<CC> {
    type AuxData = Arc<CC>;

    fn load(buffer : &mut impl Read, cc : &Self::AuxData) -> io::Result<Self> {
        let mut result = ResolutionInner::new(Arc::clone(cc));

        let algebra = result.algebra();
        let p = result.prime();
        let min_degree = result.min_degree();

        result.modules = Load::load(buffer, &(Arc::clone(&algebra), min_degree))?;
        // result.kernels = Load::load(buffer, &(min_degree, Some(p)))?;

        let max_s = result.modules.len();
        assert!(max_s > 0, "cannot load uninitialized resolution");

        let len = usize::load(buffer, &())?;
        assert_eq!(len, max_s);

        result.differentials.push(Load::load(buffer, &(result.module(0), result.zero_module(), 0))?);
        for s in 1 .. max_s as u32 {
            let d : Arc<FreeModuleHomomorphism<FreeModule<CC::Algebra>>> = Load::load(buffer, &(result.module(s), result.module(s - 1), 0))?;
            result.differentials.push(d);
        }

        let len = usize::load(buffer, &())?;
        assert_eq!(len, max_s);

        for s in 0 .. max_s as u32 {
            let c : Arc<FreeModuleHomomorphism<CC::Module>> = Load::load(buffer, &(result.module(s), result.complex().module(s), 0))?;
            result.chain_maps.push(c);
        }

        Ok(result)
    }
}

impl<CC : UnitChainComplex> Save for Resolution<CC> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        let algebra_dim = *self.next_t.lock() - self.min_degree() - 1;
        algebra_dim.save(buffer)?;
        self.inner.save(buffer)
    }
}

impl<CC : UnitChainComplex> Load for Resolution<CC> {
    type AuxData = Arc<CC>;

    fn load(buffer : &mut impl Read, cc : &Self::AuxData) -> io::Result<Self> {
        let dim = i32::load(buffer, &())?;
        cc.algebra().compute_basis(dim);

        let inner = ResolutionInner::load(buffer, cc)?;

        let result = Resolution::new_with_inner(inner, None, None);

        let next_s = result.inner.modules.len();
        assert!(next_s > 0, "Cannot load uninitialized resolution");
        let next_t = result.inner.module(0).max_computed_degree() + 1;

        result.inner.zero_module.extend_by_zero(next_t - 1);

        *result.next_s.lock() = next_s as u32;
        *result.next_t.lock() = next_t;

        Ok(result)
    }
}
