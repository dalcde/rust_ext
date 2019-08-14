use wasm_bindgen::prelude::*;

use std::rc::Rc;
use std::cell::RefCell;
use serde_json::value::Value;

use crate::algebra::{Algebra, AlgebraAny};
use crate::adem_algebra::AdemAlgebra;
use crate::milnor_algebra::MilnorAlgebra;
use crate::module::FiniteModule;
use crate::chain_complex::ChainComplexConcentratedInDegreeZero as CCDZ;
use crate::resolution::{Resolution, ModuleResolution};


// use web_sys::console;

#[wasm_bindgen]
pub struct WasmAlgebra {
    pimpl : *const AlgebraAny
}

#[wasm_bindgen]
impl WasmAlgebra {
    pub fn new_adem_algebra(p : u32, generic : bool) -> Self {
        let mut algebra = AlgebraAny::from(AdemAlgebra::new(p, generic, false));
        algebra.set_default_filtration_one_products();
        let boxed_algebra = Rc::new(algebra);
        Self {
            pimpl : Rc::into_raw(boxed_algebra)
        }
    }

    pub fn new_milnor_algebra(p : u32) -> Self {
        let mut algebra = AlgebraAny::from(MilnorAlgebra::new(p));
        algebra.set_default_filtration_one_products();
        let boxed_algebra = Rc::new(algebra);
        Self {
            pimpl : Rc::into_raw(boxed_algebra)
        }
    }

    pub fn compute_basis(&self, degree : i32) {
        self.to_algebra().compute_basis(degree);
    }

    fn to_algebra(&self) -> Rc<AlgebraAny> {
        let raw = unsafe { Rc::from_raw(self.pimpl) };
        let clone = Rc::clone(&raw);
        std::mem::forget(raw);
        clone
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}

#[wasm_bindgen]
pub struct WasmModule {
    pimpl : *const FiniteModule
}

#[wasm_bindgen]
impl WasmModule {
    pub fn new_adem_module(algebra : &WasmAlgebra, json_string : String) -> WasmModule {
        let mut json : Value = serde_json::from_str(&json_string).unwrap();
        let module = FiniteModule::from_json(algebra.to_algebra(), &mut json).ok().unwrap();
        let boxed_module = Rc::new(module);
        Self {
            pimpl : Rc::into_raw(boxed_module)
        }
    }

    fn to_module(&self) -> Rc<FiniteModule> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}


#[wasm_bindgen]
pub struct WasmCCDZ {
    pimpl : *const CCDZ<FiniteModule>
}


#[wasm_bindgen]
impl WasmCCDZ {
    pub fn new_ccdz(module : &WasmModule) -> Self {
        let cc = CCDZ::new(module.to_module());
        let boxed_cc : Rc<CCDZ<FiniteModule>> = Rc::new(cc);
        Self {
            pimpl : Rc::into_raw(boxed_cc)
        }
    }

    fn to_chain_complex(&self) -> Rc<CCDZ<FiniteModule>> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}

#[wasm_bindgen]
pub struct WasmResolution {
   pimpl : *const RefCell<ModuleResolution<FiniteModule>>
}

#[wasm_bindgen]
impl WasmResolution {
    pub fn new(chain_complex : &WasmCCDZ, json_string : String, add_class : js_sys::Function, add_structline : js_sys::Function) -> Self {
        let chain_complex = chain_complex.to_chain_complex();

        let add_class_wrapper = move |hom_deg : u32, int_deg : i32, num_gen : usize| {
            let this = JsValue::NULL;
            let js_hom_deg = JsValue::from(hom_deg);
            let js_int_deg = JsValue::from(int_deg);

            for _ in 0 .. num_gen {
                add_class.call2(&this, &js_hom_deg, &js_int_deg).unwrap();
            }
        };
        let add_class_wrapper_box = Box::new(add_class_wrapper);
        let add_stuctline_wrapper = 
            move | name : &str, 
                source_hom_deg : u32, source_int_deg : i32, source_idx : usize,
                target_hom_deg : u32, target_int_deg : i32, target_idx : usize |
        {
            let this = JsValue::NULL;
            let args_array = js_sys::Array::new();
            args_array.push(&JsValue::from(name));
            args_array.push(&JsValue::from(source_hom_deg));
            args_array.push(&JsValue::from(source_int_deg));
            args_array.push(&JsValue::from(source_idx as u32));
            args_array.push(&JsValue::from(target_hom_deg));
            args_array.push(&JsValue::from(target_int_deg));
            args_array.push(&JsValue::from(target_idx as u32));
            add_structline.apply(&this, &args_array).unwrap_throw();
        };
        let add_stuctline_wrapper_box = Box::new(add_stuctline_wrapper);
        let mut res = Resolution::new(chain_complex,  Some(add_class_wrapper_box), Some(add_stuctline_wrapper_box));

        let json : Value = serde_json::from_str(&json_string).unwrap();
        let products_value = &json["products"];
        if !products_value.is_null() {
            let products = products_value.as_array().unwrap();
            for prod in products {
                let hom_deg = prod["hom_deg"].as_u64().unwrap() as u32;
                let int_deg = prod["int_deg"].as_i64().unwrap() as i32;
                let idx = prod["index"].as_u64().unwrap() as usize;
                let name = prod["name"].as_str().unwrap();
                res.add_product(hom_deg, int_deg, idx, name.to_string());
            }
        }

        let boxed_res = Rc::new(RefCell::new(res));
        boxed_res.borrow_mut().set_self(Rc::downgrade(&boxed_res));

        let pimpl : *const RefCell<ModuleResolution<FiniteModule>> = Rc::into_raw(boxed_res);
        Self {
            pimpl
        }
    }
 
    fn to_resolution(&self) -> Rc<RefCell<ModuleResolution<FiniteModule>>> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    // pub fn step(&self, hom_deg : u32, int_deg : i32) {

    // }

    pub fn resolve_through_degree(&self, degree : i32) {
        self.to_resolution().borrow().resolve_through_degree(degree);
    }

    pub fn get_cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> String {
        self.to_resolution().borrow().get_cocycle_string(hom_deg, int_deg, idx)
    }

    pub fn free(self) {
         let _drop_me :  Rc<RefCell<ModuleResolution<FiniteModule>>>
            = unsafe { Rc::from_raw(self.pimpl) };
    }
}
