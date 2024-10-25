//!                                                                                                                                                                                           
//! This library extends the functionality provided by the `ocaml-rs` and
//! `ocaml-rs-smartptr` libraries
//! to facilitate seamless integration between Rust's async ecosystem and
//! OCaml's Lwt concurrency library.
//!                                                                                                                                                                                           
//! # Key Features                                                                                                                                                                            
//!                                                                                                                                                                                           
//! - **Domain Executor**: Provides an executor designed to run within an OCaml
//!   domain, allowing Rust async tasks to be executed while ensuring proper
//!   interaction with the OCaml runtime system.
//! - **Promise and Future Integration**: Bridges OCaml's Lwt promises with
//!   Rust's async/await syntax, enabling Rust code to await OCaml promises
//!   asynchronously.
//! - **Notification Mechanism**: Implements a notification system to wake up
//!   the OCaml event loop from Rust, ensuring efficient communication between
//!   Rust and OCaml.
//! - **OCaml Runtime Management**: Offers utilities for managing the OCaml
//!   runtime lock, ensuring safe execution of Rust code that interacts with
//!   the OCaml runtime.
//! - **Async Function Wrappers**: Provides wrappers for OCaml functions that
//!   return Lwt promises, allowing them to be called from Rust and awaited
//!   asynchronously.
//!                                                                                                                                                                                           
//! # `#[ocaml_lwt_interop::func]` Macro
//!
//! This macro helps define asynchronous Rust stubs that integrate seamlessly
//! with OCaml's Lwt concurrency library. Functions annotated with this macro
//! are transformed into asynchronous functions that return a `Promise`. Inside
//! these functions, you can use `await` to wait for asynchronous operations to
//! complete. From the end user's perspective, these functions behave like
//! `async fn` in Rust.  The macro ensures that the function's result is
//! wrapped in a `Promise`, which can be awaited in OCaml via Lwt. For
//! convenience `#[ocaml_lwt_interop::func]` macro automatically adds
//! `#[ocaml::func]` for you.
//!
//! Example:
//!
//! ```rust
//! use futures_lite::future;
//!
//! #[ocaml_lwt_interop::func]
//! pub fn my_async_func() -> () {
//!     future::yield_now().await;
//! }
//! ```
//!
//! Can be declared from OCaml side as follows:
//!
//! ```ocaml
//! external my_async_func : unit -> unit Lwt.t = "my_async_func"
//! ```
//!
//! ## Desugared version
//!
//! It is entirely possible to write async bindings using plain
//! `#[ocaml::func]`, code below is roughly what above macro-decorated version
//! expands to.
//!
//! ```rust
//! use futures_lite::future;
//! use ocaml_lwt_interop::promise::Promise;
//! use ocaml_lwt_interop::domain_executor::{ocaml_runtime, spawn_with_runtime};
//!
//! #[ocaml::func]
//! pub fn my_async_func() -> Promise<()> {
//!     let (fut, resolver) = Promise::new(gc);
//!     let task = spawn_with_runtime(gc, async move {
//!         let res = {
//!             future::yield_now().await;
//!         };
//!         let gc = ocaml_runtime();
//!         resolver.resolve(&gc, &res);
//!     });
//!     task.detach();
//!     fut
//! }
//! ```
//!
//! # Tokio integration
//!
//! `ocaml-lwt-interop` integrates Tokio natively, and manages internal Tokio
//! runtime. Below example demonstrates complex scenario with mixed tasks
//! running on OCaml domain executor and Tokio runtime.
//!
//! ```rust
//! use ocaml_lwt_interop::async_func::OCamlAsyncFunc;
//! use ocaml_lwt_interop::domain_executor;
//! use tokio::time::{sleep, Duration};
//!
//! #[ocaml_lwt_interop::func]
//! pub fn my_async_func_2(f: OCamlAsyncFunc<(), ()>) -> () {
//!     let handle = domain_executor::handle();
//!     // Spawn tokio task
//!     let join_handle = tokio::spawn(async move {
//!         // Sleep for 5 seconds
//!         sleep(Duration::from_secs(5)).await;
//!         // Spawn "OCaml-friendly" task, running on OCaml domain executor
//!         let task = handle.spawn(async move {
//!             // call `f` and await its result
//!             let res = f.call(()).await.map_err(|e| e.to_string());
//!             // we can use any Tokio functions from "OCaml-friendly" tasks
//!             sleep(Duration::from_secs(0)).await;
//!             match res {
//!                 Ok(()) => (),
//!                 Err(msg) => {
//!                     panic!("Task failed: {}", msg);
//!                 }
//!             }
//!         });
//!         // Wait for "OCaml-friendly" task to complete inside Tokio task
//!         task.await
//!     });
//!     // Wait for Tokio task to complete inside "OCaml-friendly" task
//!     join_handle.await.unwrap()
//! }
//! ```
//!
//! Correct signature for above Rust stub function for OCaml will be:
//!
//! ```ocaml
//! external my_async_func_2 : (unit -> unit Lwt.t) -> unit Lwt.t = "my_async_func_2"
//! ```

pub mod async_func;
mod caml_runtime;
pub mod domain_executor;
pub mod error;
pub mod ml_box_future;
pub mod notification;
pub mod promise;
pub mod stubs;

#[macro_use]
extern crate static_assertions;

pub use ocaml_lwt_interop_macro::func;
