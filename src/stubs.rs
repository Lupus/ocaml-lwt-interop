use ctor::ctor;
use ocaml_rs_smartptr::ptr::DynBox;
use ocaml_rs_smartptr::register_type;

use crate::bridged_executor;

///////////////////////////////////////////////////////////////////////////////
//////////                      Executor                             //////////
///////////////////////////////////////////////////////////////////////////////

#[ocaml::sig]
type Executor = bridged_executor::BridgedExecutor;

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
