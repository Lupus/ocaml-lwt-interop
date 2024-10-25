//! This module provides an implementation of a `Future` called `MlBoxFuture`,
//! which bridges OCaml's Lwt futures with Rust's async ecosystem.
//!
//! # Overview
//!
//! `MlBoxFuture` is designed to integrate OCaml's Lwt futures with Rust's
//! `async`/`await` syntax.
//! Due to limitations in passing Rust callbacks directly to OCaml functions
//! (e.g., it's not possible to call `Lwt.on_any` from Rust and pass a Rust
//! closure), the approach is inverted:
//!
//! - On the OCaml side, Lwt futures are wrapped into `MlBoxFuture` via C stubs.
//! - OCaml registers callbacks on the Lwt future to resolve or reject the
//!   `MlBoxFuture` when the Lwt future completes.
//! - This allows the Rust `MlBoxFuture` to be driven by the OCaml runtime,
//!   enabling Rust code to `await` on Lwt futures.
//!
//! # Implementation Details
//!
//! The `MlBoxFuture` is largely based on the timer future example from the Rust
//! `async` book:
//! [Wakeups](https://rust-lang.github.io/async-book/02_execution/03_wakeups.html).
//!
//! It uses an `Arc<Mutex<SharedState>>` to manage shared state between the
//! future and the code that will resolve or reject it.
//! When the future is polled, it checks if the value is available. If not, it
//! stores the waker to be woken up later.
//! When the OCaml side resolves or rejects the future, it sets the value and
//! wakes up the waker, allowing the future to complete.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use ocaml_rs_smartptr::ml_box::MlBox;

/// Shared state between the `MlBoxFuture` and the code that resolves or rejects
/// it.
///
/// This struct holds the result value (once it's available), the waker to wake
/// up the future when it's ready, and a flag indicating whether the future has
/// been completed.
#[derive(Debug)]
struct MlBoxFutureSharedState {
    /// The result value of the future, set when the future is resolved or
    /// rejected.
    value: Option<Result<MlBox, crate::error::Error>>,
    /// The waker to notify the executor when the future is ready.
    waker: Option<Waker>,
    /// Indicates whether the future has been completed.
    completed: bool,
}

/// A future that represents a computation which will eventually produce an
/// `MlBox`.
///
/// `MlBoxFuture` is used to bridge OCaml's Lwt futures with Rust's async code.
/// It can be awaited in Rust, and is resolved or rejected from the OCaml side.
#[derive(Clone, Debug)]
pub struct MlBoxFuture {
    /// Shared state between the future and the resolver.
    shared_state: Arc<Mutex<MlBoxFutureSharedState>>,
}

impl Future for MlBoxFuture {
    type Output = Result<MlBox, crate::error::Error>;

    /// Polls the future to check if it has been resolved or rejected.
    ///
    /// If the value is available, returns `Poll::Ready` with the result.
    /// Otherwise, stores the waker and returns `Poll::Pending`.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        match shared_state.value.take() {
            Some(maybe_value) => Poll::Ready(maybe_value),
            None => {
                // Store the waker so we can wake this task when the value
                // is set.
                shared_state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl MlBoxFuture {
    /// Creates a new `MlBoxFuture`.
    ///
    /// This future can later be resolved or rejected using the `resolve` or
    /// `reject` methods.
    pub fn new() -> Self {
        let shared_state = Arc::new(Mutex::new(MlBoxFutureSharedState {
            value: None,
            waker: None,
            completed: false,
        }));
        MlBoxFuture { shared_state }
    }

    /// Sets the value of the future and wakes up the waker if it's stored.
    ///
    /// This method is called internally by `resolve` and `reject`.
    ///
    /// # Panics
    ///
    /// Panics if the future has already been completed.
    fn set_value(&self, value: Result<MlBox, crate::error::Error>) {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            panic!("Attempt to resolve an already resolved promise")
        }
        shared_state.completed = true;
        shared_state.value = Some(value);
        // Wake up the waker to notify that the future is ready.
        if let Some(waker) = shared_state.waker.take() {
            waker.wake()
        }
    }

    /// Resolves the future with the given `MlBox` value.
    ///
    /// Typically called from the OCaml side when the Lwt future is fulfilled.
    pub fn resolve(&self, value: MlBox) {
        self.set_value(Ok(value))
    }

    /// Rejects the future with the given error message.
    ///
    /// Typically called from the OCaml side when the Lwt future is rejected.
    pub fn reject(&self, msg: String) {
        self.set_value(Err(crate::error::Error::LwtPromiseRejection(msg)))
    }
}

impl Default for MlBoxFuture {
    fn default() -> Self {
        Self::new()
    }
}
