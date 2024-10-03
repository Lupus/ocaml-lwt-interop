use async_task::Task;
use futures_lite::future;
use ocaml_lwt_interop::async_func::OCamlAsyncFunc;
use ocaml_lwt_interop::bridged_executor::{
    self, ocaml_runtime, spawn, spawn_using_runtime,
};
use ocaml_lwt_interop::promise::Promise;
use tokio::time::{sleep, Duration};

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
        let handle = bridged_executor::handle();
        future::yield_now().await;
        tokio::spawn(async move {
            sleep(Duration::from_secs(0)).await;
            let task = handle.spawn(async {
                let res = task.await;
                sleep(Duration::from_secs(0)).await;
                let gc = ocaml_runtime();
                match res {
                    Ok(()) => {
                        resolver.resolve(&gc, &());
                    }
                    Err(msg) => {
                        resolver.reject(&gc, format!("Task failed: {}", msg));
                    }
                }
            });
            let () = task.await;
        });
    });
    task.detach();
    fut
}
