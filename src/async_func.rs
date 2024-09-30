use std::future::IntoFuture;

use crate::bridged_executor::ocaml_runtime;
use crate::promise::{Promise, PromiseFuture};
use ocaml_gen::OCamlDesc;
use ocaml_rs_smartptr::callable::Callable;
use ocaml_rs_smartptr::func::OCamlFunc;

pub struct OCamlAsyncFunc<Args: Send, Ret: Send>(OCamlFunc<Args, Promise<Ret>>);

impl<Args, Ret> OCamlAsyncFunc<Args, Ret>
where
    Args: Send,
    Ret: Send,
{
    pub fn new(gc: &ocaml::Runtime, v: ocaml::Value) -> Self {
        OCamlAsyncFunc(OCamlFunc::new(gc, v))
    }
}

unsafe impl<Args, Ret> ocaml::FromValue for OCamlAsyncFunc<Args, Ret>
where
    Args: Send,
    Ret: Send,
{
    fn from_value(v: ocaml::Value) -> Self {
        OCamlAsyncFunc(OCamlFunc::from_value(v))
    }
}

impl<Args, Ret> OCamlAsyncFunc<Args, Ret>
where
    Args: Callable<Promise<Ret>> + Send,
    Ret: ocaml::FromValue + Send + 'static,
    Promise<Ret>: ocaml::FromValue + OCamlDesc,
{
    pub fn call(&self, args: Args) -> PromiseFuture<Ret> {
        let gc = ocaml_runtime();
        let fut = self.0.call(&gc, args);
        fut.into_future()
    }
}

impl<Args, Ret> OCamlDesc for OCamlAsyncFunc<Args, Ret>
where
    Args: Callable<Promise<Ret>> + Send,
    Ret: Send,
    Promise<Ret>: ocaml::FromValue + OCamlDesc + Send,
{
    fn ocaml_desc(env: &::ocaml_gen::Env, generics: &[&str]) -> String {
        Args::ocaml_desc(env, generics)
    }

    fn unique_id() -> u128 {
        Args::unique_id()
    }
}
