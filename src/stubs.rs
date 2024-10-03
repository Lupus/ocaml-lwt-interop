use ctor::ctor;
use ocaml_gen::{OCamlBinding, OCamlDesc};
use ocaml_rs_smartptr::ml_box::MlBox;
use ocaml_rs_smartptr::ptr::DynBox;
use ocaml_rs_smartptr::register_type;

use crate::bridged_executor::BridgedExecutor;
use crate::ml_box_future::MlBoxFuture;

pub struct PolymorphicValue<const C: char>(ocaml::Value);

impl<const C: char> ocaml_gen::OCamlDesc for PolymorphicValue<C> {
    fn ocaml_desc(_env: &ocaml_gen::Env, _generics: &[&str]) -> String {
        format!("'{}", C)
    }

    fn unique_id() -> u128 {
        panic!("unique_id is not supported for PolymorphicValue")
    }
}

unsafe impl<const C: char> ocaml::ToValue for PolymorphicValue<C> {
    fn to_value(&self, _gc: &ocaml::Runtime) -> ocaml::Value {
        self.0.clone()
    }
}

unsafe impl<const C: char> ocaml::FromValue for PolymorphicValue<C> {
    fn from_value(v: ocaml::Value) -> Self {
        Self(v)
    }
}

impl<const C: char> From<ocaml::Value> for PolymorphicValue<C> {
    fn from(value: ocaml::Value) -> Self {
        Self(value)
    }
}

impl<const C: char> Into<ocaml::Value> for PolymorphicValue<C> {
    fn into(self) -> ocaml::Value {
        self.0
    }
}

pub struct WithTypeParam<T: ocaml::FromValue + ocaml::ToValue, const C: char>(T);

impl<T: ocaml::FromValue + ocaml::ToValue + OCamlDesc, const C: char>
    WithTypeParam<T, C>
{
    pub fn new(v: T) -> Self {
        Self(v)
    }
}

impl<T: ocaml::FromValue + ocaml::ToValue + OCamlDesc, const C: char> OCamlDesc
    for WithTypeParam<T, C>
{
    fn ocaml_desc(env: &ocaml_gen::Env, generics: &[&str]) -> String {
        format!("('{} {})", C, T::ocaml_desc(env, generics))
    }

    fn unique_id() -> u128 {
        T::unique_id()
    }
}

fn insert_type_params(
    input_string: &str,
    type_params: &str,
) -> Result<String, &'static str> {
    let type_nonrec = "type nonrec ";

    if let Some(type_index) = input_string.find(type_nonrec) {
        let insert_index = type_index + type_nonrec.len();
        let mut result = String::from(&input_string[..insert_index]);
        result.push_str(type_params);
        result.push_str(" ");
        result.push_str(&input_string[insert_index..]);
        Ok(result)
    } else {
        Err("Could not find 'type nonrec' in the input string")
    }
}

impl<T: ocaml::FromValue + ocaml::ToValue + OCamlBinding + OCamlDesc, const C: char>
    OCamlBinding for WithTypeParam<T, C>
{
    fn ocaml_binding(
        env: &mut ::ocaml_gen::Env,
        rename: Option<&'static str>,
        new_type: bool,
    ) -> String {
        let ty_id = Self::unique_id();

        if new_type {
            let orig = T::ocaml_binding(env, rename, new_type);
            insert_type_params(&orig, format!("'{}", C).as_str()).unwrap()
        } else {
            let name = Self::ocaml_desc(env, &[]);
            let ty_name = rename.expect("bug in ocaml-gen: rename should be Some");
            env.add_alias(ty_id, ty_name);

            format!("type nonrec '{} {} = '{} {}", C, ty_name, C, name)
        }
    }
}

unsafe impl<T: ocaml::FromValue + ocaml::ToValue, const C: char> ocaml::ToValue
    for WithTypeParam<T, C>
{
    fn to_value(&self, gc: &ocaml::Runtime) -> ocaml::Value {
        self.0.to_value(gc)
    }
}

unsafe impl<T: ocaml::FromValue + ocaml::ToValue, const C: char> ocaml::FromValue
    for WithTypeParam<T, C>
{
    fn from_value(v: ocaml::Value) -> Self {
        Self(T::from_value(v))
    }
}

impl<T: ocaml::FromValue + ocaml::ToValue, const C: char> From<T>
    for WithTypeParam<T, C>
{
    fn from(value: T) -> Self {
        Self(value)
    }
}

///////////////////////////////////////////////////////////////////////////////
//////////                       Promise                             //////////
///////////////////////////////////////////////////////////////////////////////

pub type Future = WithTypeParam<DynBox<MlBoxFuture>, 'a'>;

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_mlbox_future_create() -> Future {
    Future::new(MlBoxFuture::new().into())
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_mlbox_future_resolve(fut: Future, value: PolymorphicValue<'a'>) {
    fut.0.coerce().resolve(MlBox::new(gc, value.into()));
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn lwti_mlbox_future_reject(fut: Future, msg: String) {
    fut.0.coerce().reject(msg);
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

#[ctor]
fn register_rtti_2() {
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
