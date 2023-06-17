use crate::local_executor;
use crate::ptr::CamlRef;
use crate::util::{ensure_rooted_value, ambient_gc};

///////////////////////////////////////////////////////////////////////////////
//////////                       Promise                             //////////
///////////////////////////////////////////////////////////////////////////////

#[ocaml::sig]
type Promise<T> = crate::promise::Promise<T>;

#[ocaml::func]
#[ocaml::sig("unit -> promise")]
pub fn lwti_promise_create() -> CamlRef<Promise<()>> {
    Promise::<()>::new().into()
}

#[ocaml::func]
#[ocaml::sig("promise -> 'a -> unit")]
pub fn lwti_promise_resolve(promise: CamlRef<Promise<()>>, value: ocaml::Value) {
    let value = ensure_rooted_value(value);
    promise.resolve(value)
}

#[ocaml::func]
#[ocaml::sig("promise -> exn -> unit")]
pub fn lwti_promise_reject(promise: CamlRef<Promise<()>>, exn: ocaml::Value) {
    let exn = ensure_rooted_value(exn);
    promise.reject(exn)
}

///////////////////////////////////////////////////////////////////////////////
//////////                      Executor                             //////////
///////////////////////////////////////////////////////////////////////////////

#[ocaml::sig]
type Executor = local_executor::LocalExecutor;

#[ocaml::func]
#[ocaml::sig("int -> executor")]
pub fn lwti_executor_create(notify_id: isize) -> CamlRef<Executor> {
    let mut executor = Executor::new();
    executor.set_notifier(crate::notification::Notification(notify_id));
    executor.into()
}

#[ocaml::func]
#[ocaml::sig("executor -> unit")]
pub fn lwti_executor_run_pending(executor: CamlRef<Executor>) {
    while executor.try_tick() {}
}

#[ocaml::func]
#[ocaml::sig("executor -> (int -> promise) -> unit")]
pub fn lwti_executor_test(executor: CamlRef<Executor>, f: ocaml::Value) {
    let f = ensure_rooted_value(f);

    let task = executor.spawn(async move {
        let mut page_nb = 0;
        let gc = ambient_gc();
        let f_callable = ocaml::function!(f, (n: ocaml::Int) -> CamlRef<Promise<()>>);
        loop {
            f_callable(&gc, &page_nb).unwrap().clone().await.unwrap();
            page_nb = page_nb + 1;
        }
    });
    task.detach();
}
