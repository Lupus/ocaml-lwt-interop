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

ocaml::import! {
    fn olwti_lwt_task() -> (ocaml::Value, ocaml::Value);
    fn olwti_lwt_wakeup_later(resolver: ocaml::Value, value: ocaml::Value) -> Result<(),String>;
    fn olwti_lwt_wakeup_later_exn(resolver: ocaml::Value, msg: String) -> Result<(), String>;
    fn olwti_wrap_lwt_future(fut: ocaml::Value) -> DynBox<MlBoxFuture>;
}

pub struct Resolver<T>
where
    T: ocaml::ToValue,
{
    resolver: MlBox,
    _marker: AssertUnwindSafe<PhantomData<T>>,
}

/* As Resolver is a wraper on top of MlBox, we mark Resolver as Send + Sync as
 * MlBox itself */
unsafe impl<T: ocaml::ToValue> Send for Resolver<T> {}
unsafe impl<T: ocaml::ToValue> Sync for Resolver<T> {}

assert_impl_all!(Resolver<ocaml::Value>: Send, Sync, UnwindSafe, RefUnwindSafe);

impl<T: ocaml::ToValue> Resolver<T> {
    pub fn resolve(self, gc: &ocaml::Runtime, v: &T) {
        let resolver = self.resolver.as_value(gc);
        unsafe { olwti_lwt_wakeup_later(gc, resolver, v.to_value(gc)) }
            .expect("olwti_lwt_wakeup_later has thrown an exception")
            .unwrap()
    }

    pub fn reject(self, gc: &ocaml::Runtime, msg: String) {
        let resolver = self.resolver.as_value(gc);
        unsafe { olwti_lwt_wakeup_later_exn(gc, resolver, msg) }
            .expect("olwti_lwt_wakeup_later_exn has thrown an exception")
            .unwrap()
    }
}

#[derive(Debug)]
pub struct Promise<T> {
    inner: MlBox,
    _marker: AssertUnwindSafe<PhantomData<T>>,
}

/* As Promise is a wraper on top of MlBox, we mark Promise as Send + Sync as
 * MlBox itself */
unsafe impl<T> Send for Promise<T> {}
unsafe impl<T> Sync for Promise<T> {}

assert_impl_all!(Promise<ocaml::Value>: Send, Sync, UnwindSafe, RefUnwindSafe);

impl<T> Promise<T>
where
    T: ocaml::ToValue,
{
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
    fn ocaml_desc(env: &::ocaml_gen::Env, generics: &[&str]) -> String {
        format!("(({}) Lwt.t)", T::ocaml_desc(env, generics))
    }

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

pub struct PromiseFuture<T> {
    promise: Option<MlBox>,
    state: PromiseFutureState<T>,
}

assert_impl_all!(PromiseFuture<()>: Send, Unpin);

enum PromiseFutureState<T> {
    NotStarted,
    Running(Pin<Box<dyn Future<Output = Result<T, crate::error::Error>> + Send>>),
    Completed,
}

impl<T> PromiseFuture<T>
where
    T: ocaml::FromValue + Send + 'static,
{
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        loop {
            match &mut this.state {
                PromiseFutureState::NotStarted => {
                    let gc = ocaml_runtime();
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

                    let future = Box::pin(async move {
                        let ml_box = ml_box_future.await?;
                        let gc = ocaml_runtime();
                        let value = ml_box.into_value(&gc)
                            .expect("MlBox from ml_box_future.await? is expected to be only reference");
                        Ok(T::from_value(value))
                    });

                    this.state = PromiseFutureState::Running(future);
                }
                PromiseFutureState::Running(fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(result) => {
                        this.state = PromiseFutureState::Completed;
                        return Poll::Ready(result);
                    }
                    Poll::Pending => return Poll::Pending,
                },
                PromiseFutureState::Completed => {
                    panic!("PromiseFuture polled after completion")
                }
            }
        }
    }
}
