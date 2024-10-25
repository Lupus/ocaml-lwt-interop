//!                                                                                                                                                                                           
//! The `Promise`, `Resolver`` and `PromiseFuture` types in this module allow
//! Rust code to await OCaml promises asynchronously. This is achieved by
//! bridging OCaml's Lwt promises with Rust's `Future` trait.
//!                                                                                                                                                                                           

use crate::{domain_executor::ocaml_runtime, ml_box_future::MlBoxFuture};
use highway::{HighwayHash, HighwayHasher};
use ocaml_gen::{const_random, OCamlDesc};
use ocaml_rs_smartptr::ml_box::MlBox;
use ocaml_rs_smartptr::ptr::DynBox;
use std::{
    future::{Future, IntoFuture},
    hash::Hash,
    marker::PhantomData,
    panic::{AssertUnwindSafe, RefUnwindSafe, UnwindSafe},
    pin::Pin,
    task::{Context, Poll},
};

// OCaml callbacks are registered in ../lib/Rust_async.ml
ocaml::import! {
    // `olwti_lwt_task` calls `Lwt.task`, and returns promise and resolver
    fn olwti_lwt_task() -> (ocaml::Value, ocaml::Value);
    // `olwti_lwt_wakeup_later` calls `Lwt.wakeup_later`
    fn olwti_lwt_wakeup_later(resolver: ocaml::Value, value: ocaml::Value) -> Result<(), String>;
    // `olwti_lwt_wakeup_later_exn` calls `Lwt.wakeup_later_exn`
    fn olwti_lwt_wakeup_later_exn(resolver: ocaml::Value, msg: String) -> Result<(), String>;
    // `olwti_wrap_lwt_future` creates new `MlBoxFuture`, and links
    // resolution/rejection of `fut` (which is `'a Lwt.t``) to corresponding
    // `MlBoxFuture`
    fn olwti_wrap_lwt_future(fut: ocaml::Value) -> DynBox<MlBoxFuture>;
}

/// `Resolver<T>` is a wrapper around ocaml::Value which is `'a Lwt.u``,
/// where `'a == T`
pub struct Resolver<T>
where
    T: ocaml::ToValue,
{
    resolver: MlBox,
    _marker: AssertUnwindSafe<PhantomData<T>>,
}

// As Resolver is a wraper on top of MlBox, we mark Resolver as Send + Sync as
// MlBox itself
unsafe impl<T: ocaml::ToValue> Send for Resolver<T> {}
unsafe impl<T: ocaml::ToValue> Sync for Resolver<T> {}

assert_impl_all!(Resolver<ocaml::Value>: Send, Sync, UnwindSafe, RefUnwindSafe);

impl<T: ocaml::ToValue> Resolver<T> {
    /// Resolves the `'a Lwt.u` via `Lwt.wakeup_later`
    pub fn resolve(self, gc: &ocaml::Runtime, v: &T) {
        let resolver = self.resolver.as_value(gc);
        unsafe { olwti_lwt_wakeup_later(gc, resolver, v.to_value(gc)) }
            .expect("olwti_lwt_wakeup_later has thrown an exception")
            .unwrap()
    }

    /// Rejects the `'a Lwt.u` via `Lwt.wakeup_later_exn`
    pub fn reject(self, gc: &ocaml::Runtime, msg: String) {
        let resolver = self.resolver.as_value(gc);
        unsafe { olwti_lwt_wakeup_later_exn(gc, resolver, msg) }
            .expect("olwti_lwt_wakeup_later_exn has thrown an exception")
            .unwrap()
    }
}

/// `Promise<T>` is a wrapper around ocaml::Value which is `'a Lwt.t``,
/// where `'a == T`
#[derive(Debug)]
pub struct Promise<T> {
    inner: MlBox,
    _marker: AssertUnwindSafe<PhantomData<T>>,
}

// As Promise is a wraper on top of MlBox, we mark Promise as Send + Sync as
// MlBox itself
unsafe impl<T> Send for Promise<T> {}
unsafe impl<T> Sync for Promise<T> {}

assert_impl_all!(Promise<ocaml::Value>: Send, Sync, UnwindSafe, RefUnwindSafe);

impl<T> Promise<T>
where
    T: ocaml::ToValue,
{
    /// Creates a new promise/resolver pair, calls `Lwt.task` under the hood
    pub fn new(gc: &ocaml::Runtime) -> (Promise<T>, Resolver<T>) {
        let (v_fut, v_resolver) = unsafe { olwti_lwt_task(gc) }
            .expect("olwti_lwt_task has thrown an exception");
        let fut: Promise<T> = Promise {
            inner: MlBox::new(gc, v_fut),
            _marker: AssertUnwindSafe(PhantomData),
        };
        let resolver: Resolver<T> = Resolver {
            resolver: MlBox::new(gc, v_resolver),
            _marker: AssertUnwindSafe(PhantomData),
        };
        (fut, resolver)
    }
}

unsafe impl<T> ocaml::ToValue for Promise<T>
where
    T: ocaml::ToValue,
{
    fn to_value(&self, gc: &ocaml::Runtime) -> ocaml::Value {
        self.inner.as_value(gc)
    }
}

unsafe impl<T> ocaml::FromValue for Promise<T>
where
    T: ocaml::FromValue + ocaml::ToValue,
{
    fn from_value(v: ocaml::Value) -> Self {
        /* from_value should really receive runtime handle :shrug: */
        /* let's just assume that no one is going to call from_value manually on
         * a weird thread... */
        let gc = unsafe { ocaml::Runtime::recover_handle() };
        Self {
            inner: MlBox::new(gc, v),
            _marker: AssertUnwindSafe(PhantomData),
        }
    }
}

impl<T> IntoFuture for Promise<T>
where
    T: ocaml::FromValue + Send + 'static,
{
    type Output = Result<T, crate::error::Error>;
    type IntoFuture = PromiseFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        PromiseFuture::new(self)
    }
}

impl<T> OCamlDesc for Promise<T>
where
    T: OCamlDesc,
{
    /// Wraps underlying OCaml type with `'a Lwt.t`
    fn ocaml_desc(env: &::ocaml_gen::Env, generics: &[&str]) -> String {
        format!("(({}) Lwt.t)", T::ocaml_desc(env, generics))
    }

    /// Hashes underlying unique_id with unique key
    fn unique_id() -> u128 {
        let key = highway::Key([
            const_random!(u64),
            const_random!(u64),
            const_random!(u64),
            const_random!(u64),
        ]);
        let mut hasher = HighwayHasher::new(key);
        T::unique_id().hash(&mut hasher);
        let result = hasher.finalize128();
        (result[0] as u128) | ((result[1] as u128) << 64)
    }
}

/// `PromiseFuture<T>` bridges a `Promise<T>` (an OCaml promise) with Rust's
/// `Future` trait.
/// It allows Rust code to await an OCaml promise asynchronously.
///
/// Keeping `PromiseFuture` separate from `Promise<T>` is important for several reasons:
/// - **Ownership and Mutability**: It manages the asynchronous state without
///   requiring mutable access to `Promise<T>`.
/// - **Separation of Concerns**: `Promise<T>` represents the promise itself,
///   while `PromiseFuture<T>` handles its execution.
/// - **State Management**: It maintains internal states (`NotStarted`,
///   `Running`, `Completed`) necessary for the `Future` trait.
///
/// Therefore, we avoid implementing `Future` directly on `Promise<T>` to keep
/// concerns separated and the code maintainable.
pub struct PromiseFuture<T> {
    /// Holds the OCaml promise; taken when the future starts.
    promise: Option<MlBox>,
    /// Manages the internal state of the future.
    state: PromiseFutureState<T>,
}

// Ensures that `PromiseFuture` is `Send` and `Unpin`.
assert_impl_all!(PromiseFuture<()>: Send, Unpin);

/// Represents the different states during the future's lifecycle.
enum PromiseFutureState<T> {
    /// Initial state before starting.
    NotStarted,
    /// Running state, holds the pinned future.
    Running(Pin<Box<dyn Future<Output = Result<T, crate::error::Error>> + Send>>),
    /// State after completion.
    Completed,
}

impl<T> PromiseFuture<T>
where
    T: ocaml::FromValue + Send + 'static,
{
    /// Creates a new `PromiseFuture` from a `Promise<T>`.
    fn new(promise: Promise<T>) -> Self {
        Self {
            promise: Some(promise.inner.clone()),
            state: PromiseFutureState::NotStarted,
        }
    }
}

impl<T> Future for PromiseFuture<T>
where
    T: ocaml::FromValue + Send + 'static,
{
    type Output = Result<T, crate::error::Error>;

    /// `PromiseFuture<T>` must only be polled from a task, which is running on
    /// [OCaml domain executor](crate::domain_executor::DomainExecutor), or
    /// where it's context has been
    /// [entered](crate::domain_executor::DomainExecutor::enter).
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Obtain a mutable reference to self.
        let this = self.get_mut();

        loop {
            match &mut this.state {
                // Initialize the future if not started.
                PromiseFutureState::NotStarted => {
                    let gc = ocaml_runtime();
                    // Wrap the OCaml promise into a future that can be awaited.
                    let wrapper = unsafe {
                        olwti_wrap_lwt_future(
                            &gc,
                            this.promise
                                .take()
                                .expect("Promise does not have a value inside")
                                .into_value(&gc)
                                .expect("MlBox inside PromiseFuture is expected to be only reference"),
                        )
                    }
                    .expect("olwti_wrap_lwt_future has thrown an exception");
                    let ml_box_future = wrapper.coerce().clone();

                    // Create a Rust future to await the OCaml future and process the result.
                    let future = Box::pin(async move {
                        let ml_box = ml_box_future.await?;
                        let gc = ocaml_runtime();
                        let value = ml_box.into_value(&gc)
                            .expect("MlBox from ml_box_future.await? is expected to be only reference");
                        Ok(T::from_value(value))
                    });

                    // Transition to the running state.
                    this.state = PromiseFutureState::Running(future);
                }
                // Poll the running future.
                PromiseFutureState::Running(fut) => match fut.as_mut().poll(cx) {
                    // On completion, update state and return the result.
                    Poll::Ready(result) => {
                        this.state = PromiseFutureState::Completed;
                        return Poll::Ready(result);
                    }
                    // If still pending, return `Poll::Pending`.
                    Poll::Pending => return Poll::Pending,
                },
                // Panic if polled after completion.
                PromiseFutureState::Completed => {
                    panic!("PromiseFuture polled after completion")
                }
            }
        }
    }
}
