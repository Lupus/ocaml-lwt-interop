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
