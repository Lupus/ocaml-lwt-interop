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

///////////////////////////////////////////////////////////////////////////////
//////////               OCaml bindings generation                   //////////
///////////////////////////////////////////////////////////////////////////////

ocaml_gen_bindings! {
    decl_module!("Tests", {
        decl_func!(lwti_tests_bench => "bench");
        decl_func!(lwti_tests_test1 => "test_1");
        decl_func!(lwti_tests_test2 => "test_2");
        decl_func!(lwti_tests_test_sync_call => "test_sync_call");
    });
}
