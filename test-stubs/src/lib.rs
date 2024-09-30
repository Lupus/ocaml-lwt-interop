use async_task::Task;
use futures_lite::future;
use ocaml_lwt_interop::async_func::OCamlAsyncFunc;
use ocaml_lwt_interop::bridged_executor::{ocaml_runtime, spawn, spawn_using_runtime};
use ocaml_lwt_interop::promise::Promise;

#[ocaml::func]
pub fn lwti_tests_bench() -> Promise<()> {
    let (fut, resolver) = Promise::new(gc);
    let task = spawn_using_runtime(gc, async move {
        future::yield_now().await;
        resolver.resolve(&ocaml_runtime(), &());
    });
    task.detach();
    fut
}

#[ocaml::func]
pub fn lwti_tests_test1() -> Promise<()> {
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

#[ocaml::func]
pub fn lwti_tests_test2(f: OCamlAsyncFunc<(), ()>) -> Promise<()> {
    let (fut, resolver) = Promise::new(gc);
    let task: Task<Result<(), String>> = spawn_using_runtime(gc, async move {
        future::yield_now().await;
        f.call(()).await.map_err(|e| e.to_string())?;
        Ok(())
    });
    let task = spawn_using_runtime(gc, async move {
        match task.await {
            Ok(()) => {
                resolver.resolve(&ocaml_runtime(), &());
            }
            Err(msg) => {
                let gc = &ocaml_runtime();
                resolver.reject(gc, format!("Task failed: {}", msg));
            }
        }
    });
    task.detach();
    fut
}
