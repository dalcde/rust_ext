use fp::prime::ValidPrime;
use fp::vector::FpVector;
use crate::algebra::Algebra;
use crate::algebra::Bialgebra;
use crate::algebra::AdemAlgebra;
use crate::algebra::MilnorAlgebra;

use enum_dispatch::enum_dispatch;
use serde::Deserialize;
use serde_json::Value;

// This is here so that the Python bindings can use modules defined for SteenrodAlgebraT with their own algebra enum.
// In order for things to work SteenrodAlgebraT cannot implement Algebra.
// Otherwise, the algebra enum for our bindings will see an implementation clash.
pub trait SteenrodAlgebraT : Send + Sync + 'static + Algebra {
    fn to_steenrod_algebra(&self) -> SteenrodAlgebraBorrow;
}

pub enum SteenrodAlgebraBorrow<'a> {
    BorrowAdem(&'a AdemAlgebra),
    BorrowMilnor(&'a MilnorAlgebra),
}

#[enum_dispatch(Algebra)]
pub enum SteenrodAlgebra {
    AdemAlgebra,
    MilnorAlgebra,
}

impl SteenrodAlgebraT for SteenrodAlgebra {
    fn to_steenrod_algebra(&self) -> SteenrodAlgebraBorrow {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => SteenrodAlgebraBorrow::BorrowAdem(a),
            SteenrodAlgebra::MilnorAlgebra(a) => SteenrodAlgebraBorrow::BorrowMilnor(a),
        }
    }
}




impl Bialgebra for SteenrodAlgebra {
    fn decompose (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize)> {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => a.decompose(op_deg, op_idx),
            SteenrodAlgebra::MilnorAlgebra(a) => a.decompose(op_deg, op_idx),
        }
    }

    fn coproduct (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize, i32, usize)> {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => a.coproduct(op_deg, op_idx),
            SteenrodAlgebra::MilnorAlgebra(a) => a.coproduct(op_deg, op_idx),
        }
    }
}

#[derive(Deserialize, Debug)]
struct MilnorProfileOption {
    truncated : Option<bool>,
    q_part : Option<u32>,
    p_part : Option<Vec<u32>>
}

#[derive(Deserialize, Debug)]
struct AlgebraSpec {
    p : u32,
    algebra : Option<Vec<String>>,
    profile : Option<MilnorProfileOption>
}

impl SteenrodAlgebra {
    pub fn from_json(json : &Value, mut algebra_name : String) -> error::Result<SteenrodAlgebra> {
        let spec : AlgebraSpec = serde_json::from_value(json.clone())?;

        let p = ValidPrime::try_new(spec.p)
            .ok_or_else(|| error::GenericError::new(format!("Invalid prime: {}", spec.p)))?;

        if let Some(mut list) = spec.algebra {
            if !list.contains(&algebra_name) {
                println!("Module does not support algebra {}", algebra_name);
                println!("Using {} instead", list[0]);
                algebra_name = list.remove(0);
            }
        }

        let algebra : SteenrodAlgebra;
        match algebra_name.as_ref() {
            "adem" => algebra = SteenrodAlgebra::AdemAlgebra(AdemAlgebra::new(p, *p != 2, false)),
            "milnor" => {
                let mut algebra_inner = MilnorAlgebra::new(p);
                if let Some(profile) = spec.profile {
                    if let Some(truncated) = profile.truncated {
                        algebra_inner.profile.truncated = truncated;
                    }
                    if let Some(q_part) = profile.q_part {
                        algebra_inner.profile.q_part = q_part;
                    }
                    if let Some(p_part) = profile.p_part {
                        algebra_inner.profile.p_part = p_part;
                    }
                }
                algebra = SteenrodAlgebra::MilnorAlgebra(algebra_inner);
            }
            _ => { return Err(InvalidAlgebraError { name : algebra_name }.into()); }
        };
        Ok(algebra)
    }

    pub fn to_json(&self, json: &mut Value) {
        match self {
            SteenrodAlgebra::MilnorAlgebra(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::from(a.generic);

                if !a.profile.is_trivial() {
                    json["algebra"] = Value::from(vec!["milnor"]);
                    json["profile"] = Value::Object(serde_json::map::Map::with_capacity(3));
                    if a.profile.truncated {
                        json["profile"]["truncated"] = Value::Bool(true);
                    }
                    if a.profile.q_part != !0 {
                        json["profile"]["q_part"] = Value::from(a.profile.q_part);
                    }
                    if !a.profile.p_part.is_empty() {
                        json["profile"]["p_part"] = Value::from(a.profile.p_part.clone());
                    }
                }
            }
            SteenrodAlgebra::AdemAlgebra(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::Bool(a.generic);
            }
        }
    }
}

#[derive(Debug)]
struct InvalidAlgebraError {
    name : String
}

impl std::fmt::Display for InvalidAlgebraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid algebra: {}", &self.name)
    }
}

impl std::error::Error for InvalidAlgebraError {
    fn description(&self) -> &str {
        "Invalid algebra supplied"
    }
}
