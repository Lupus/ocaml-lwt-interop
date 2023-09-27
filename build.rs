pub fn main() -> std::io::Result<()> {
    ocaml_build::Sigs::new("lib/stubs.ml").generate()
}
