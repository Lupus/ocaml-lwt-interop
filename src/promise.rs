use crate::borrow_mut;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::{future::Future, pin::Pin, task::Context, task::Poll, task::Waker};

// Promise is largely based on timer future example from async book:
// https://rust-lang.github.io/async-book/02_execution/03_wakeups.html

#[derive(Debug)]
struct PromiseSharedState {
    value: Option<Result<ocaml::Value, ocaml::Error>>,
    waker: Option<Waker>,
    completed: bool,
}

#[derive(Clone, Debug)]
pub struct Promise<T>
where
    T: ocaml::FromValue,
{
    shared_state: Rc<RefCell<PromiseSharedState>>,
    marker: PhantomData<T>,
}

impl<T> Future for Promise<T>
where
    T: ocaml::FromValue,
{
    type Output = Result<T, ocaml::Error>; // Specify the output type of your future

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = borrow_mut!(self.shared_state);
        match shared_state.value.take() {
            Some(maybe_value) => Poll::Ready(maybe_value.map(|x| ocaml::FromValue::from_value(x))),
            None => {
                shared_state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<T> Promise<T>
where
    T: ocaml::FromValue,
{
    pub fn new() -> Self {
        let shared_state = Rc::new(RefCell::new(PromiseSharedState {
            value: None,
            waker: None,
            completed: false,
        }));
        Promise {
            shared_state,
            marker: PhantomData,
        }
    }

    fn set_value(&self, value: Result<ocaml::Value, ocaml::Error>) {
        let mut shared_state = borrow_mut!(self.shared_state);
        if shared_state.completed {
            panic!("Attempt to resolve an already resolved promise")
        }
        shared_state.completed = true;
        shared_state.value = Some(value);
        if let Some(waker) = shared_state.waker.take() {
            waker.wake()
        }
    }

    pub fn resolve(&self, value: ocaml::Value) {
        self.set_value(Ok(value))
    }

    pub fn reject(&self, exn: ocaml::Value) {
        self.set_value(Err(ocaml::Error::Caml(ocaml::CamlError::Exception(exn))))
    }
}

impl<T> Default for Promise<T>
where
    T: ocaml::FromValue,
{
    fn default() -> Self {
        Self::new()
    }
}
