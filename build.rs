use std::env;

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/stubs.rs");
    let current_dir = env::current_dir()?;
    let current_dir = current_dir.to_str().unwrap();
    let out_filename = "stubs.ml";
    if current_dir.ends_with("/_build/default") {
        println!(
            "cargo:warning=[ocaml-lwt-interop/build.rs] Not generating {} as launched from dune build dir",
            out_filename
        );
        return Ok(());
    }
    ocaml_build::Sigs::new(out_filename).generate()
}
