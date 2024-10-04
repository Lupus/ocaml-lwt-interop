use ocaml_gen::prelude::*;
use ocaml_lwt_interop_test_stubs::*;

use std::fmt::Write as _;
use std::io;
use std::io::Write;

fn main() -> std::io::Result<()> {
    ocaml_rs_smartptr::registry::initialize_plugins();
    let mut w = String::new();
    let env = &mut Env::new();

    ocaml_gen::decl_module!(w, env, "Tests", {
        ocaml_gen::decl_func!(w, env, lwti_tests_bench => "bench");
        ocaml_gen::decl_func!(w, env, lwti_tests_test1 => "test_1");
        ocaml_gen::decl_func!(w, env, lwti_tests_test2 => "test_2");
    });

    io::stdout().write_all(w.as_bytes())?;
    Ok(())
}
