use async_task::Task;
use futures_lite::future;
use ocaml_lwt_interop::async_func::OCamlAsyncFunc;
use ocaml_lwt_interop::bridged_executor::{
    self, ocaml_runtime, run_with_gc_lock, spawn, spawn_using_runtime,
};
use ocaml_lwt_interop::promise::Promise;
use ocaml_rs_smartptr::func::OCamlFunc;
use tokio::time::{sleep, Duration};

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_bench() -> () {
    future::yield_now().await;
    resolver.resolve(&ocaml_runtime(), &());
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_test1() {
    future::yield_now().await;
    spawn(async {
        future::yield_now().await;
        resolver.resolve(&ocaml_runtime(), &());
    })
    .await
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_test2(f: OCamlAsyncFunc<(), ()>) -> () {
    let task: Task<Result<(), String>> = spawn(async move {
        future::yield_now().await;
        f.call(()).await.map_err(|e| e.to_string())?;
        Ok(())
    });
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
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_tests_test_sync_call(f: OCamlFunc<(), ()>) {
    let handle = bridged_executor::handle();
    let join_handle = tokio::spawn(async move {
        run_with_gc_lock(&handle, move |gc| f.call(gc, ()));
    });
    join_handle.await.unwrap();
    resolver.resolve(&ocaml_runtime(), &());
}
