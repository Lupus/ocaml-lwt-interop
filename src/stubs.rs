use ocaml_rs_smartptr::ml_box::MlBox;
use ocaml_rs_smartptr::ocaml_gen_bindings;
use ocaml_rs_smartptr::ocaml_gen_extras::{PolymorphicValue, WithTypeParams, P1};
use ocaml_rs_smartptr::ptr::DynBox;
use ocaml_rs_smartptr::{register_rtti, register_type};

use crate::bridged_executor::BridgedExecutor;
use crate::ml_box_future::MlBoxFuture;

///////////////////////////////////////////////////////////////////////////////
//////////                       Promise                             //////////
///////////////////////////////////////////////////////////////////////////////

pub type Future = WithTypeParams<P1<'a'>, DynBox<MlBoxFuture>>;

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_mlbox_future_create() -> Future {
    MlBoxFuture::new().into()
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_mlbox_future_resolve(fut: Future, value: PolymorphicValue<'a'>) {
    fut.coerce().resolve(MlBox::new(gc, value.into()));
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_mlbox_future_reject(fut: Future, msg: String) {
    fut.coerce().reject(msg);
}

///////////////////////////////////////////////////////////////////////////////
//////////                      Executor                             //////////
///////////////////////////////////////////////////////////////////////////////

pub type Executor = DynBox<BridgedExecutor>;

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_executor_create(notify_id: isize) -> Executor {
    let executor = BridgedExecutor::new(crate::notification::Notification(notify_id));
    DynBox::new_shared(executor)
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_executor_run_pending(executor: Executor) {
    let ex = executor.coerce();
    ex.tick();
}

///////////////////////////////////////////////////////////////////////////////
//////////               Register Types & Traits                     //////////
///////////////////////////////////////////////////////////////////////////////

register_rtti! {
    register_type!(
        {
            ty: crate::bridged_executor::BridgedExecutor,
            marker_traits: [core::marker::Sync, core::marker::Send],
            object_safe_traits: [],
        }
    );
    register_type!(
        {
            ty: crate::ml_box_future::MlBoxFuture,
            marker_traits: [core::marker::Sync, core::marker::Send],
            object_safe_traits: [],
        }
    );
}

///////////////////////////////////////////////////////////////////////////////
//////////               OCaml bindings generation                   //////////
///////////////////////////////////////////////////////////////////////////////

ocaml_gen_bindings! {
    decl_module!("Future", {
        decl_type!(Future => "t");
        decl_func!(lwti_mlbox_future_create => "create");
        decl_func!(lwti_mlbox_future_resolve => "resolve");
        decl_func!(lwti_mlbox_future_reject => "reject");
    });

    decl_module!("Executor", {
        decl_type!(Executor => "t");
        decl_func!(lwti_executor_create => "create");
        decl_func!(lwti_executor_run_pending => "run_pending");
    });
}
