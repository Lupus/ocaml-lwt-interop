use crate::{
    bridged_executor::ocaml_runtime,
    ml_box::{MlBox, MlBoxFuture},
    util::ExportedRoot,
};
use ocaml_rs_smartptr::{ptr::DynBox, util::ensure_rooted_value};
use std::{
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

ocaml::import! {
    fn olwti_lwt_task() -> (ocaml::Value, ocaml::Value);
    fn olwti_lwt_wakeup_later(resolver: ocaml::Value, value: ocaml::Value);
    fn olwti_lwt_wakeup_later_exn(resolver: ocaml::Value, msg: String);
    fn olwti_wrap_lwt_future(fut: ocaml::Value) -> DynBox<MlBoxFuture>;
}

pub struct Resolver<T>
where
    T: ocaml::ToValue,
{
    resolver: ExportedRoot,
    marker: PhantomData<T>,
}

impl<T: ocaml::ToValue> Resolver<T> {
    pub fn resolve(self, gc: &ocaml::Runtime, v: &T) {
        let resolver = self.resolver.into_value(gc);
        unsafe { olwti_lwt_wakeup_later(gc, resolver, v.to_value(gc)) }
            .expect("olwti_lwt_wakeup_later has thrown an exception")
    }

    pub fn reject(self, gc: &ocaml::Runtime, msg: String) {
        let resolver = self.resolver.into_value(gc);
        unsafe { olwti_lwt_wakeup_later_exn(gc, resolver, msg) }
            .expect("olwti_lwt_wakeup_later_exn has thrown an exception")
    }
}

pub struct Promise<T> {
    inner: ocaml::Value,
    marker: PhantomData<T>,
}

impl<T> Promise<T>
where
    T: ocaml::ToValue,
{
    pub fn new(gc: &ocaml::Runtime) -> (Promise<T>, Resolver<T>) {
        let (v_fut, v_resolver) = unsafe { olwti_lwt_task(gc) }
            .expect("olwti_lwt_task has thrown an exception");
        let fut: Promise<T> = Promise {
            inner: ensure_rooted_value(v_fut),
            marker: PhantomData,
        };
        let resolver: Resolver<T> = Resolver {
            resolver: ExportedRoot::new(gc, v_resolver),
            marker: PhantomData,
        };
        (fut, resolver)
    }
}

unsafe impl<T> ocaml::ToValue for Promise<T>
where
    T: ocaml::ToValue,
{
    fn to_value(&self, _gc: &ocaml::Runtime) -> ocaml::Value {
        self.inner.clone()
    }
}

unsafe impl<T> ocaml::FromValue for Promise<T>
where
    T: ocaml::FromValue + ocaml::ToValue,
{
    fn from_value(v: ocaml::Value) -> Self {
        Self {
            inner: ensure_rooted_value(v),
            marker: PhantomData,
        }
    }
}

// impl<T> Promise<T>
// where
//     T: ocaml::FromValue + Send + 'static,
// {
//     pub fn as_future(
//         &self,
//         gc: &ocaml::Runtime,
//     ) -> Box<dyn Future<Output = Result<T, crate::error::Error>>> {
//         let wrapper = unsafe { olwti_wrap_lwt_future(gc, self.inner.clone()) }
//             .expect("olwti_wrap_lwt_future has thrown an exception");
//         let ml_box_future = wrapper.coerce().clone();
//         let task = spawn_using_runtime(gc, async move {
//             let ml_box = ml_box_future.await?;
//             let gc = ocaml_runtime();
//             Ok(T::from_value(ml_box.into_value(&gc)))
//         });
//         Box::new(task)
//     }
// }

impl<T> IntoFuture for Promise<T>
where
    T: ocaml::FromValue + Send + 'static,
{
    type Output = Result<T, crate::error::Error>;
    type IntoFuture = PromiseFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        let gc = ocaml_runtime();
        PromiseFuture::new(&gc, self)
    }
}

pub struct PromiseFuture<T> {
    promise: Option<MlBox>,
    state: PromiseFutureState<T>,
}

enum PromiseFutureState<T> {
    NotStarted,
    Running(Pin<Box<dyn Future<Output = Result<T, crate::error::Error>> + Send>>),
    Completed,
}

impl<T> PromiseFuture<T>
where
    T: ocaml::FromValue + Send + 'static,
{
    fn new(gc: &ocaml::Runtime, promise: Promise<T>) -> Self {
        Self {
            promise: Some(MlBox::new(gc, promise.inner)),
            state: PromiseFutureState::NotStarted,
        }
    }
}

impl<T> Future for PromiseFuture<T>
where
    T: ocaml::FromValue + Send + 'static,
{
    type Output = Result<T, crate::error::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };

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
                                .into_value(&gc),
                        )
                    }
                    .expect("olwti_wrap_lwt_future has thrown an exception");
                    let ml_box_future = wrapper.coerce().clone();

                    let future = Box::pin(async move {
                        let ml_box = ml_box_future.await?;
                        let gc = ocaml_runtime();
                        Ok(T::from_value(ml_box.into_value(&gc)))
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
