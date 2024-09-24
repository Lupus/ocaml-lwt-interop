use ocaml::ToValue;
use ocaml_rs_smartptr::util::ensure_rooted_value;

use std::marker::PhantomData;

use crate::util::ExportedRoot;

ocaml::import! {
    fn olwti_lwt_task() -> (ocaml::Value, ocaml::Value);
    fn olwti_lwt_wakeup_later(resolver: ocaml::Value, value: ocaml::Value);
    fn olwti_lwt_wakeup_later_exn(resolver: ocaml::Value, exn: ocaml::Value);
}

pub struct Promise<T>
where
    T: ocaml::ToValue,
{
    inner: ocaml::Value,
    marker: PhantomData<T>,
}

unsafe impl<T> ocaml::ToValue for Promise<T>
where
    T: ocaml::ToValue,
{
    fn to_value(&self, _gc: &ocaml::Runtime) -> ocaml::Value {
        self.inner.clone()
    }
}

pub struct Resolver<T>
where
    T: ocaml::ToValue,
{
    resolver: ExportedRoot,
    marker: PhantomData<T>,
}

pub fn new<T: ocaml::ToValue>(gc: &ocaml::Runtime) -> (Promise<T>, Resolver<T>) {
    let (v_fut, v_resolver) =
        unsafe { olwti_lwt_task(gc) }.expect("olwti_lwt_task has thrown an exception");
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

impl<T: ocaml::ToValue> Resolver<T> {
    pub fn resolve(self, gc: &ocaml::Runtime, v: &T) {
        let resolver = self.resolver.into_value(gc);
        unsafe { olwti_lwt_wakeup_later(gc, resolver, v.to_value(gc)) }
            .expect("olwti_lwt_wakeup_later has thrown an exception")
    }

    pub fn reject(self, gc: &ocaml::Runtime, error: impl std::error::Error) {
        let resolver = self.resolver.into_value(gc);
        unsafe {
            olwti_lwt_wakeup_later_exn(gc, resolver, error.to_string().to_value(gc))
        }
        .expect("olwti_lwt_wakeup_later_exn has thrown an exception")
    }
}
