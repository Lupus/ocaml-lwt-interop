use async_task::Task;
use futures_lite::future;
use ocaml_lwt_interop::bridged_executor::{ocaml_runtime, spawn, spawn_using_runtime};
use ocaml_lwt_interop::ml_box::MlBox;
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

#[ocaml::func]
pub fn lwti_executor_test2(f: ocaml::Value) -> Promise<()> {
    let f = MlBox::new(gc, f);
    let (fut, resolver) = Promise::new(gc);
    let task: Task<Result<(), String>> = spawn_using_runtime(gc, async move {
        future::yield_now().await;
        let fut = {
            let gc = &ocaml_runtime();
            let f = ocaml::function!(f.into_value(gc), (_unit:()) -> Promise<()>);
            let fut = f(gc, &())
                .map_err(|e| format!("OCaml callback raised exception: {:?}", e))?;
            fut
        };
        fut.await.map_err(|e| e.to_string())?;
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
