use rust_ext::Config;
use rust_ext::construct;
use rust_ext::matrix::Matrix;
use rust_ext::module::Module;
use rust_ext::fp_vector::{FpVectorT, FpVector};
use rust_ext::algebra::Algebra;

#[test]
fn extend_identity() {
    check_algebra("S_2", 30, "adem");
    check_algebra("S_3", 50, "adem");
    check_algebra("Calpha", 50, "adem");
    check_algebra("S_2", 30, "milnor");
    check_algebra("S_3", 50, "milnor");
    check_algebra("Calpha", 50, "milnor");
    check_algebra("tmf2", 40, "milnor");
}

fn check_algebra (module_name : &str, max_degree : i32, algebra_name: &str) {
    println!("module : {}", module_name);
    let path = std::path::PathBuf::from("static/modules");
    let a = Config {
        module_paths : vec![path.clone()],
        module_file_name : module_name.to_string(),
        max_degree,
        algebra_name : String::from(algebra_name)
    };

    let bundle = construct(&a).unwrap();
    let p = bundle.algebra.prime();

    bundle.resolution.borrow_mut().add_self_map(0, 0, "id".to_string(), Matrix::from_vec(p, &[vec![1]]));

    let resolution = bundle.resolution.borrow();

    resolution.resolve_through_degree(max_degree);

    for s in 0 ..= max_degree as u32 {
        let map = resolution.self_maps[0].map.get_map(s);
        let source = resolution.get_module(s);
        for t in 0..= max_degree {
            for idx in 0 .. source.get_number_of_gens_in_degree(t){
                let mut correct_result = FpVector::new(p, source.get_dimension(t));
                correct_result.set_entry(source.operation_generator_to_index(0, 0, t, idx), 1);
                // Mathematically, there is no reason these should be lietrally
                // equal.
                assert_eq!(map.get_output(t, idx), &correct_result);
            }
        }
    }
}