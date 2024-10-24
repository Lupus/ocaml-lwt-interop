use std::{
    ffi::c_int,
    panic::{catch_unwind, UnwindSafe},
    process::abort,
};

extern "C" {
    /*
    caml_acquire_runtime_system() The calling thread re-acquires the master lock
    and other Caml resources. It may block until no other thread uses the Caml
    run-time system.
    */
    // fn caml_acquire_runtime_system();

    /*
    caml_release_runtime_system() The calling thread releases the master lock
    and other Caml resources, enabling other threads to run Caml code in
    parallel with the execution of the calling thread.
    */
    // fn caml_release_runtime_system();

    /* caml_enter_blocking_section as an alias for caml_release_runtime_system */
    fn caml_enter_blocking_section();

    /* caml_leave_blocking_section as an alias for caml_acquire_runtime_system */
    fn caml_leave_blocking_section();

    /*
    caml_c_thread_register() registers the calling thread with the Caml run-time
    system. Returns 1 on success, 0 on error. Registering an already-register
    thread does nothing and returns 0.
    */
    fn caml_c_thread_register() -> c_int;

    /*
    caml_c_thread_unregister() must be called before the thread terminates, to
    unregister it from the Caml run-time system. Returns 1 on success, 0 on
    error. If the calling thread was not previously registered, does nothing and
    returns 0.
    */
    fn caml_c_thread_unregister() -> c_int;
}

unsafe fn caml_acquire_runtime_system() {
    caml_leave_blocking_section();
}

unsafe fn caml_release_runtime_system() {
    caml_enter_blocking_section();
}

pub(crate) fn register_thread() {
    if unsafe { caml_c_thread_register() } != 1 {
        eprintln!("caml_c_thread_register() failed!");
        abort()
    }
}

pub(crate) fn unregister_thread() {
    if unsafe { caml_c_thread_unregister() } != 1 {
        eprintln!("caml_c_thread_unregister() failed!");
        abort()
    }
}

pub(crate) fn with_released_lock<F, R>(f: F) -> R
where
    F: FnOnce() -> R + UnwindSafe,
{
    unsafe { caml_release_runtime_system() };
    let result = catch_unwind(f);
    unsafe { caml_acquire_runtime_system() };
    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

pub(crate) fn with_acquired_lock<F, R>(f: F) -> R
where
    F: FnOnce(&ocaml::Runtime) -> R + UnwindSafe,
{
    unsafe { caml_acquire_runtime_system() };
    let gc = unsafe { ocaml::Runtime::recover_handle() };
    let result = catch_unwind(|| f(gc));
    unsafe { caml_release_runtime_system() };
    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
