use futures_lite::future;
use ocaml_lwt_interop::bridged_executor::{ocaml_runtime, spawn, spawn_using_runtime};
use ocaml_lwt_interop::promise::Promise;

#[ocaml::func]
pub fn lwti_executor_bench() -> Promise<()> {
    let (fut, resolver) = Promise::new(gc);
    let task = spawn_using_runtime(gc, async move {
        future::yield_now().await;
        resolver.resolve(&ocaml_runtime(), &());
    });
    task.detach();
    fut
}

#[ocaml::func]
pub fn lwti_executor_test() -> Promise<()> {
    let (fut, resolver) = Promise::new(gc);
    let task = spawn_using_runtime(gc, async move {
        future::yield_now().await;
        spawn(async {
            future::yield_now().await;
            resolver.resolve(&ocaml_runtime(), &());
        })
        .await
    });
    task.detach();
    fut
}
