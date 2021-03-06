use crate::prime::ValidPrime;
use crate::vector::{FpVector, FpVectorT};
use super::{
    Matrix,
    Subspace
};

/// Given a matrix M, a quasi-inverse Q is a map from the co-domain to the domain such that xQM = x
/// for all x in the image (recall our matrices act on the right).
///
/// # Fields
///  * `image` - The image of the original matrix. If the image is omitted, it is assumed to be
///  everything (with the standard basis).
///  * `preimage` - The actual quasi-inverse, where the basis of the image is that given by
///  `image`.
#[derive(Debug, Clone)]
pub struct QuasiInverse {
    pub image : Option<Subspace>,
    pub preimage : Matrix
}


impl QuasiInverse {
    pub fn prime(&self) -> ValidPrime {
        self.preimage.prime()
    }

    /// Apply the quasi-inverse to an input vector and add a constant multiple of the result
    /// to an output vector
    ///
    /// # Arguments
    ///  * `target` - The output vector
    ///  * `coeff` - The constant multiple above
    ///  * `input` - The input vector, expressed in the basis of the ambient space
    pub fn apply(&self, target : &mut FpVector, coeff : u32, input : &FpVector){
        let p = self.prime();
        let mut row = 0;
        let columns = input.dimension();
        for i in 0 .. columns {
            if let Some(image) = &self.image { if image.pivots()[i] < 0 {
                continue;
            }}
            let c = input.entry(i);
            if c != 0 {
                target.add(&self.preimage[row], (coeff * c) % *p);
            }
            row += 1;
        }
    }
}
