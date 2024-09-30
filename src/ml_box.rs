use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

#[derive(Debug)]
pub struct MlBox(ocaml::root::Root);

unsafe impl Send for MlBox {}

impl MlBox {
    pub fn new(_gc: &ocaml::Runtime, value: ocaml::Value) -> Self {
        match value {
            ocaml::Value::Raw(v) => Self(unsafe { ocaml::root::Root::new(v) }),
            ocaml::Value::Root(r) => Self(r),
        }
    }

    pub fn into_value(self, _gc: &ocaml::Runtime) -> ocaml::Value {
        ocaml::Value::Root(self.0)
    }
}

// Promise is largely based on timer future example from async book:
// https://rust-lang.github.io/async-book/02_execution/03_wakeups.html

#[derive(Debug)]
struct MlBoxFutureSharedState {
    value: Option<Result<MlBox, crate::error::Error>>,
    waker: Option<Waker>,
    completed: bool,
}

#[derive(Clone, Debug)]
pub struct MlBoxFuture {
    shared_state: Arc<Mutex<MlBoxFutureSharedState>>,
}

impl Future for MlBoxFuture {
    type Output = Result<MlBox, crate::error::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        match shared_state.value.take() {
            Some(maybe_value) => Poll::Ready(maybe_value),
            None => {
                shared_state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl MlBoxFuture {
    pub fn new() -> Self {
        let shared_state = Arc::new(Mutex::new(MlBoxFutureSharedState {
            value: None,
            waker: None,
            completed: false,
        }));
        MlBoxFuture { shared_state }
    }

    fn set_value(&self, value: Result<MlBox, crate::error::Error>) {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.completed {
            panic!("Attempt to resolve an already resolved promise")
        }
        shared_state.completed = true;
        shared_state.value = Some(value);
        if let Some(waker) = shared_state.waker.take() {
            waker.wake()
        }
    }

    pub fn resolve(&self, value: MlBox) {
        self.set_value(Ok(value))
    }

    pub fn reject(&self, msg: String) {
        self.set_value(Err(crate::error::Error::LwtPromiseRejection(msg)))
    }
}

impl Default for MlBoxFuture {
    fn default() -> Self {
        Self::new()
    }
}
