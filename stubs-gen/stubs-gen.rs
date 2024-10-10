#[allow(clippy::single_component_path_imports)]
#[allow(unused_imports)]
use rust_staticlib_rust_async::ocaml_lwt_interop;

#[allow(clippy::single_component_path_imports)]
#[allow(unused_imports)]
use rust_staticlib_rust_async::ocaml_lwt_interop_test_stubs;

fn main() -> std::io::Result<()> {
    ocaml_rs_smartptr::ocaml_gen_extras::stubs_gen_main()
}
