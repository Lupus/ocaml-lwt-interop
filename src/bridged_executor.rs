// Good read on async streams, executors, reactors and tasks:
// https://www.qovery.com/blog/a-guided-tour-of-streams-in-rust

use crate::notification::Notification;
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Context,
};

use async_executor::{Executor, Task};

use future_local_storage::{FutureLocalStorage, FutureOnceCell};
use ocaml_rs_smartptr::ptr::DynBox;

ocaml::import! {
    fn olwti_current_executor() -> DynBox<BridgedExecutor>;
}

pub struct LwtExecutorBridge {
    fut: Pin<Box<dyn Future<Output = ()> + Sync + Send + 'static>>,
    notification: Notification,
}

impl LwtExecutorBridge {
    fn new(ex: Arc<Executor<'static>>, notification: Notification) -> Self {
        Self {
            fut: Box::pin(async move {
                println!("+++ running ex till pending future");
                ex.run(futures_lite::future::pending::<()>()).await;
                println!("--- running ex till pending future");
            }),
            notification,
        }
    }

    pub fn tick(&mut self) {
        let notification = self.notification;
        let waker = waker_fn::waker_fn(move || notification.send());
        let mut cx = Context::from_waker(&waker);
        let _ = self.fut.as_mut().poll(&mut cx);
    }
}

static CONTEXT: FutureOnceCell<ExecutorContext> = FutureOnceCell::new();

struct ExecutorContext {
    ex: Arc<Executor<'static>>,
}

impl ExecutorContext {
    // Each future has its own executor context in which it is executed, and after execution
    // is complete, we just ignore this context
    pub async fn in_scope<R, F>(ex: Arc<Executor<'static>>, future: F) -> R
    where
        F: Future<Output = R>,
    {
        let (_this, result) = future.with_scope(&CONTEXT, Self::new(ex)).await;
        result
    }

    fn new(ex: Arc<Executor<'static>>) -> Self {
        
        Self { ex }
    }

    fn with<R, F: FnOnce(&Self) -> R + std::panic::UnwindSafe>(scope: F) -> R {
        if let Ok(res) = std::panic::catch_unwind(|| CONTEXT.with(|ctx| scope(ctx))) {
            return res;
        }
        panic!("No ExecutionContext is registered within the current task")
    }
}

pub struct BridgedExecutor {
    pub ex: Arc<Executor<'static>>,
    pub bridge: Mutex<LwtExecutorBridge>,
}

impl BridgedExecutor {
    pub fn new(notification: Notification) -> BridgedExecutor {
        let ex = Arc::new(Executor::new());
        let bridge = Mutex::new(LwtExecutorBridge::new(ex.clone(), notification));
        BridgedExecutor { ex, bridge }
    }

    pub fn tick(&self) {
        let mut bridge = self.bridge.lock().unwrap();
        bridge.tick();
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.ex
            .spawn(ExecutorContext::in_scope(self.ex.clone(), future))
    }
}

pub fn spawn<T>(future: impl Future<Output = T> + Send + 'static) -> Task<T>
where
    T: Send + 'static,
{
    let ex = ExecutorContext::with(|ctx| ctx.ex.clone());
    ex.spawn(ExecutorContext::in_scope(ex.clone(), future))
}

pub struct OcamlRuntimeGuard<'a> {
    _marker: PhantomData<&'a ()>,
    _marker2: PhantomData<std::rc::Rc<()>>,
}

impl<'a> std::ops::Deref for OcamlRuntimeGuard<'a> {
    type Target = ocaml::Runtime;

    fn deref(&self) -> &'a Self::Target {
        unsafe { ocaml::Runtime::recover_handle() }
    }
}

pub fn ocaml_runtime<'a>() -> OcamlRuntimeGuard<'a> {
    /* Ensure we're running in a task which is driven by our executor (which is
     * in turn `tick`-ed only from the same OCaml domain) */
    let () = ExecutorContext::with(|_ctx| ());
    OcamlRuntimeGuard {
        _marker: PhantomData,
        _marker2: PhantomData,
    }
}

pub fn spawn_using_runtime<T>(
    gc: &ocaml::Runtime,
    future: impl Future<Output = T> + Send + 'static,
) -> Task<T>
where
    T: Send + 'static,
{
    let ex = unsafe { olwti_current_executor(gc) }
        .expect("olwti_current_executor has thrown an exception");
    ex.coerce().spawn(future)
}

pub struct Handle {
    ex: Arc<Executor<'static>>,
}

impl Handle {
    fn new(ex: Arc<Executor<'static>>) -> Self {
        Self { ex }
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.ex
            .spawn(ExecutorContext::in_scope(self.ex.clone(), future))
    }
}

pub fn handle() -> Handle {
    let ex = ExecutorContext::with(|ctx| ctx.ex.clone());
    Handle::new(ex)
}
