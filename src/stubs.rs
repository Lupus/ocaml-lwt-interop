use crate::local_executor;
use crate::promise::Promise;
use crate::util::ambient_gc;
use ctor::ctor;
use futures_lite::future;
use ocaml_rs_smartptr::ptr::DynBox;
use ocaml_rs_smartptr::register_type;

///////////////////////////////////////////////////////////////////////////////
//////////                      Executor                             //////////
///////////////////////////////////////////////////////////////////////////////

#[ocaml::sig]
type Executor = local_executor::BridgedExecutor;

#[ocaml::func]
#[ocaml::sig("int -> executor")]
pub fn lwti_executor_create(notify_id: isize) -> DynBox<Executor> {
    let executor = Executor::new(crate::notification::Notification(notify_id));
    DynBox::new_shared(executor)
}

#[ocaml::func]
#[ocaml::sig("executor -> unit")]
pub fn lwti_executor_run_pending(executor: DynBox<Executor>) {
    let ex = executor.coerce();
    ex.tick();
}

#[ocaml::func]
#[ocaml::sig("executor -> unit Lwt.t")]
pub fn lwti_executor_test(executor: DynBox<Executor>) -> Promise<()> {
    let (fut, resolver) = crate::promise::new(gc);
    let ex = executor.coerce();
    let task = ex.spawn(async move {
        let gc = ambient_gc();
        future::yield_now().await;
        resolver.resolve(gc, &());
    });
    task.detach();
    fut
}

// Register supported traits for types that we bind
#[ctor]
fn register_rtti() {
    register_type!(
        {
            ty: crate::stubs::Executor,
            marker_traits: [core::marker::Sync, core::marker::Send],
            object_safe_traits: [],
        }
    );
}
