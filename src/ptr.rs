use ocaml::interop::{DynBox, OCaml};
use std::{borrow::Borrow, fmt::Debug, marker::PhantomData, ops::Deref};

#[derive(Debug)]
pub struct CamlRef<'a, T>
where
    T: Debug,
{
    dynbox_root: ocaml::root::Root,
    ptr: *const T,
    marker: PhantomData<&'a T>,
}

impl<'a, T> CamlRef<'a, T>
where
    T: Debug + 'static,
{
    pub fn new(dynbox: OCaml<DynBox<T>>) -> Self {
        let dynbox_root = unsafe { ocaml::root::Root::new(dynbox.get_raw()) };
        let ptr: &T = dynbox.borrow();
        Self {
            dynbox_root,
            ptr: ptr as *const T,
            marker: PhantomData,
        }
    }
}

impl<'a, T> Deref for CamlRef<'a, T>
where
    T: Debug + Sized,
{
    type Target = T;

    fn deref(&self) -> &'a Self::Target {
        unsafe { &*self.ptr }
    }
}

unsafe impl<'a, T> ocaml::FromValue for CamlRef<'a, T>
where
    T: Debug + 'static,
{
    fn from_value(v: ocaml::Value) -> Self {
        let dynbox: OCaml<DynBox<T>> = v.into();
        CamlRef::new(dynbox)
    }
}

unsafe impl<'a, T> ocaml::ToValue for CamlRef<'a, T>
where
    T: Debug + 'static,
{
    fn to_value(&self, _rt: &ocaml::Runtime) -> ocaml::Value {
        unsafe { self.dynbox_root.get() }.into()
    }
}

impl<'a, T> From<T> for CamlRef<'a, T>
where
    T: Debug + 'static,
{
    fn from(value: T) -> Self {
        let gc = unsafe { ocaml::interop::OCamlRuntime::recover_handle() };
        let dynbox = OCaml::box_value(gc, value);
        CamlRef::new(dynbox)
    }
}

pub type CamlRet<T> = CamlRef<'static, T>;
