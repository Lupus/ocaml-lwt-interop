use ocaml_gen::prelude::*;
use ocaml_lwt_interop::stubs::*;

use std::fmt::Write as _;
use std::io;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut w = String::new();
    let env = &mut Env::new();

    ocaml_gen::decl_module!(w, env, "Future", {
        ocaml_gen::decl_type!(w, env, Future => "t");
        ocaml_gen::decl_func!(w, env, lwti_mlbox_future_create => "create");
        ocaml_gen::decl_func!(w, env, lwti_mlbox_future_resolve => "resolve");
        ocaml_gen::decl_func!(w, env, lwti_mlbox_future_reject => "reject");
    });

    io::stdout().write_all(w.as_bytes())?;
    Ok(())
}
