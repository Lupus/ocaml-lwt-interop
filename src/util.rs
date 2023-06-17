// Ensures that ocaml value is wrapped into boxroot, if you plan to store
// incoming OCaml function arguments elsewhere so that they outlive their scope,
// you need to ensure they are rooted, otherwise OCaml GC will collect them
// behind your back!
pub fn ensure_rooted_value(value: ocaml::Value) -> ocaml::Value {
    match value {
        ocaml::Value::Raw(v) => unsafe { ocaml::Value::Root(ocaml::root::Root::new(v)) },
        ocaml::Value::Root(_) => value,
    }
}

// Provides static reference to "ambient" OCaml Runtime. It is safe to use this
// function when your application is actually an OCaml one, and you extend it
// with Rust, because in this case you have a guarantee that OCaml runtime is
// always initialized and outlives any Rust code.
pub fn ambient_gc() -> &'static mut ocaml::Runtime {
    unsafe { ocaml::interop::OCamlRuntime::recover_handle() }
}

#[macro_export]
macro_rules! borrow {
    ($var:expr) => {{
        let borrow_result = $var.try_borrow();
        match borrow_result {
            Ok(value) => {
                // println!("Borrow {} at {}:{}", stringify!($var), file!(), line!(),);
                value
            }
            Err(err) => panic!(
                "Borrow error at {}:{}. Failed to borrow variable '{}': {}",
                file!(),
                line!(),
                stringify!($var),
                err
            ),
        }
    }};
}

#[macro_export]
macro_rules! borrow_mut {
    ($var:expr) => {{
        let borrow_result = $var.try_borrow_mut();
        match borrow_result {
            Ok(value) => {
                // println!("Borrow mut {} at {}:{}", stringify!($var), file!(), line!(),);
                value
            }
            Err(err) => panic!(
                "Borrow error at {}:{}. Failed to mutably borrow variable '{}': {}",
                file!(),
                line!(),
                stringify!($var),
                err
            ),
        }
    }};
}
