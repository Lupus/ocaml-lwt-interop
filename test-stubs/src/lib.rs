use async_task::Task;
use futures_lite::future;
use ocaml_lwt_interop::async_func::OCamlAsyncFunc;
use ocaml_lwt_interop::domain_executor::{self, run_in_ocaml_domain, spawn};
use ocaml_rs_smartptr::func::OCamlFunc;
use ocaml_rs_smartptr::ocaml_gen_bindings;
use tokio::time::{sleep, Duration};

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_bench() -> () {
    future::yield_now().await;
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_test1() {
    future::yield_now().await;
    spawn(async {
        future::yield_now().await;
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
    let handle = domain_executor::handle();
    future::yield_now().await;
    let join_handle = tokio::spawn(async move {
        sleep(Duration::from_secs(0)).await;
        let task = handle.spawn(async {
            let res = task.await;
            sleep(Duration::from_secs(0)).await;
            match res {
                Ok(()) => (),
                Err(msg) => {
                    panic!("Task failed: {}", msg);
                }
            }
        });
        task.await
    });
    join_handle.await.unwrap()
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_tests_test_sync_call(f: OCamlFunc<(), ()>) {
    let handle = domain_executor::handle();
    let join_handle = tokio::spawn(async move {
        unsafe { run_in_ocaml_domain(&handle, move |gc| f.call(gc, ())) };
    });
    join_handle.await.unwrap();
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_tests_spawn_lwt(val: i64) -> ocaml_lwt_interop::promise::Promise<i64> {
    ocaml_lwt_interop::domain_executor::spawn_lwt(gc, async move {
        future::yield_now().await;
        val + 1
    })
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_run_in_ocaml_domain(f: OCamlFunc<(), ()>) -> () {
    let handle = domain_executor::handle();
    let join_handle = tokio::spawn(async move {
        future::yield_now().await;
        unsafe { run_in_ocaml_domain(&handle, move |gc| f.call(gc, ())) };
    });
    join_handle.await.unwrap();
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_handle(f: OCamlAsyncFunc<(), ()>) -> () {
    let handle = domain_executor::handle();
    let join_handle = tokio::spawn(async move {
        future::yield_now().await;
        let task =
            handle.spawn(async move { f.call(()).await.map_err(|e| e.to_string()) });
        task.await.expect("ocaml task failed");
    });
    join_handle.await.unwrap();
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_tests_promise_create(val: i64) -> ocaml_lwt_interop::promise::Promise<i64> {
    use ocaml_lwt_interop::domain_executor::{ocaml_runtime, spawn_with_runtime};
    let (promise, resolver) = ocaml_lwt_interop::promise::Promise::new(gc);
    let task = spawn_with_runtime(gc, async move {
        future::yield_now().await;
        let gc = &ocaml_runtime();
        resolver.resolve(gc, &val);
    });
    task.detach();
    promise
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_tests_promise_create_err(
    msg: String,
) -> ocaml_lwt_interop::promise::Promise<i64> {
    use ocaml_lwt_interop::domain_executor::{ocaml_runtime, spawn_with_runtime};
    let (promise, resolver) = ocaml_lwt_interop::promise::Promise::new(gc);
    let task = spawn_with_runtime(gc, async move {
        future::yield_now().await;
        let gc = &ocaml_runtime();
        resolver.reject(gc, msg);
    });
    task.detach();
    promise
}

#[ocaml_lwt_interop::func]
#[ocaml_gen::func]
pub fn lwti_tests_await_promise(
    p: ocaml_lwt_interop::promise::Promise<i64>,
) -> Result<i64, String> {
    p.await.map_err(|e| e.to_string())
}

///////////////////////////////////////////////////////////////////////////////
//////////               OCaml bindings generation                   //////////
///////////////////////////////////////////////////////////////////////////////

ocaml_gen_bindings! {
    decl_module!("Tests", {
        decl_func!(lwti_tests_bench => "bench");
        decl_func!(lwti_tests_test1 => "test_1");
        decl_func!(lwti_tests_test2 => "test_2");
        decl_func!(lwti_tests_test_sync_call => "test_sync_call");
        decl_func!(lwti_tests_spawn_lwt => "spawn_lwt");
        decl_func!(lwti_tests_run_in_ocaml_domain => "run_in_ocaml_domain");
        decl_func!(lwti_tests_handle => "handle_test");
        decl_func!(lwti_tests_promise_create => "promise_create");
        decl_func!(lwti_tests_promise_create_err => "promise_create_err");
        decl_func!(lwti_tests_await_promise => "await_promise");
    });
}
