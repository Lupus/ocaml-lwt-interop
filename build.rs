fn main() -> std::io::Result<()> {
    /* Generation of OCaml .ml/.mli is conditional on
     * OCAML_BUILD_GENERATE_SIGNATURES environment variable, which is set in
     * corresponding dune rule, launching cargo. This avoids generation when
     * cargo is launched by IDE in source dir */
    match std::env::var("OCAML_BUILD_GENERATE_SIGNATURES") {
        Ok(v) => match v.as_str() {
            "OCAML_LWT_INTEROP" => ocaml_build::Sigs::new("rust.ml").generate(),
            _ => Result::Ok(()),
        },
        Err(_) => Result::Ok(()),
    }
}
