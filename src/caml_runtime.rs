//! This module provides some low-level primities for managing OCaml domain
//! lock. Tested on OCaml 4, should work on OCaml 5 just fine.
//!
//! # OCaml 5 Runtime System Acquisition
//
//! OCaml 5 introduced true parallelism through domains, which map to OS threads
//! and allow OCaml code to run simultaneously on multiple CPU cores. However,
//! this new model raises questions about how existing C code and thread pools
//! interact with OCaml domains. Specifically, there is uncertainty around which
//! OCaml domain will be used when C code calls functions like
//! `caml_acquire_runtime_system`` to interact with the OCaml runtime. This is
//! particularly relevant for scenarios where C thread pools are created from
//! OCaml and need to execute OCaml code.
//!
//! ## Runtime System Functions
//!
//! In OCaml 5, `caml_acquire_runtime_system` and `caml_release_runtime_system`
//! operate on a per-domain basis, acquiring or releasing the lock for the
//! current domain associated with the calling OS thread[1]. These functions
//! ensure thread-safety when C code interacts with the OCaml runtime. For
//! backwards compatibility, they are still used in OCaml 5, but newer functions
//! like `caml_domain_lock` and `caml_domain_unlock` are recommended for new
//! code as they are more explicit about operating on the current domain[1].
//! It's crucial to maintain proper balance between acquire and release calls
//! within each execution path in C code to prevent deadlocks or other
//! synchronization issues.
//!
//! ## C Thread Registration
//!
//! When creating C thread pools that need to interact with the OCaml runtime,
//! each worker thread must call `caml_c_thread_register` to associate itself
//! with the OCaml runtime system[2]. This registration process does not create
//! a new OCaml domain, but rather allows the C thread to safely interact with
//! existing domains. After registration, when these threads call
//! `caml_acquire_runtime_system`, they will always acquire the lock for domain
//! 0 (the main domain)[1]. This means all registered C threads compete for the
//! same domain lock, potentially limiting parallelism in scenarios with
//! multiple OCaml domains.
//!
//! ## Domain and Thread Interaction
//!
//! When a C thread pool is created from OCaml via a C stub, and worker threads
//! call `caml_c_thread_register`, they become associated with the OCaml runtime
//! but not with any specific domain. Subsequent calls to
//! `caml_acquire_runtime_system` or `caml_domain_lock` from these threads will
//! always acquire the lock for domain 0 (the main domain)[1]. This design
//! choice means that all worker threads compete for the same domain lock,
//! potentially limiting parallelism in multi-domain scenarios[1]. The
//! association between a C thread and an OCaml domain is determined once at
//! thread creation, and currently, the only way to create threads on non-zero
//! domains is from OCaml code[1][3].
//!
//! ## References
//!
//! [1] <https://discuss.ocaml.org/t/test-caml-state-and-conditionally-caml-acquire-runtime-system-good-or-bad/12489>
//! [2] <https://dev.to/yawaramin/practical-ocaml-multicore-edition-3gf2>
//! [3] <https://ocaml.org/manual/5.2/parallelism.html>

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
    // fn caml_acquire_runtime_system(); // For some reason this symbol is missing...

    /*
    caml_release_runtime_system() The calling thread releases the master lock
    and other Caml resources, enabling other threads to run Caml code in
    parallel with the execution of the calling thread.
    */
    // fn caml_release_runtime_system(); // For some reason this symbol is missing...

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

/// Registeres current thread with OCaml runtime, aborts the program if
/// registration failed
pub(crate) fn register_thread() {
    if unsafe { caml_c_thread_register() } != 1 {
        eprintln!("caml_c_thread_register() failed!");
        abort()
    }
}

/// Un-registeres current thread with OCaml runtime, aborts the program if
/// un-registration failed
pub(crate) fn unregister_thread() {
    if unsafe { caml_c_thread_unregister() } != 1 {
        eprintln!("caml_c_thread_unregister() failed!");
        abort()
    }
}

/// Runs `f` with OCaml domain lock being released. `f` **MUST NOT** use any
/// functions that require OCaml domain lock to be held.
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

/// Runs `f` with OCaml domain lock being acquired. `f` can safely use any
/// functions that require OCaml domain lock to be held.
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
