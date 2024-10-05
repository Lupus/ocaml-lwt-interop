// Good read on async streams, executors, reactors and tasks:
// https://www.qovery.com/blog/a-guided-tour-of-streams-in-rust

use crate::notification::Notification;
use std::{
    cell::RefCell,
    ffi::c_int,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    process::abort,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex, OnceLock, Weak,
    },
    task::{Context, Waker},
};

use async_executor::{Executor, Task};
use futures_lite::future::yield_now;
use tokio::runtime::Builder;

use ocaml_rs_smartptr::ptr::DynBox;

extern "C" {
    /*
    caml_acquire_runtime_system() The calling thread re-acquires the master lock
    and other Caml resources. It may block until no other thread uses the Caml
    run-time system.
    */
    // fn caml_acquire_runtime_system();

    /*
    caml_release_runtime_system() The calling thread releases the master lock
    and other Caml resources, enabling other threads to run Caml code in
    parallel with the execution of the calling thread.
    */
    // fn caml_release_runtime_system();

    /* caml_enter_blocking_section as an alias for caml_release_runtime_system */
    fn caml_enter_blocking_section();

    /* caml_leave_blocking_section as an alias for caml_acquire_runtime_system */
    fn caml_leave_blocking_section();

    /*
    caml_c_thread_register() registers the calling thread with the Caml run-time
    system. Returns 1 on success, 0 on error. Registering an already-register
    thread does nothing and returns 0.
    */
    fn caml_c_thread_register() -> c_int;

    /*
    caml_c_thread_unregister() must be called before the thread terminates, to
    unregister it from the Caml run-time system. Returns 1 on success, 0 on
    error. If the calling thread was not previously registered, does nothing and
    returns 0.
    */
    fn caml_c_thread_unregister() -> c_int;
}

unsafe fn caml_acquire_runtime_system() {
    caml_leave_blocking_section();
}

unsafe fn caml_release_runtime_system() {
    caml_enter_blocking_section();
}


fn global_tokio_runtime() -> Arc<tokio::runtime::Runtime> {
    static RT: OnceLock<Mutex<Weak<tokio::runtime::Runtime>>> = OnceLock::new();
    let mut weak_rt = RT.get_or_init(|| Mutex::new(Weak::new())).lock().unwrap();
    match weak_rt.upgrade() {
        Some(rt) => rt,
        None => {
            let new_rt = Arc::new(
                Builder::new_multi_thread()
                    .enable_all()
                    .thread_name_fn(|| {
                        static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                        let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                        format!("olwti-tokio-worker-{}", id)
                    })
                    .on_thread_start(|| {
                        if unsafe { caml_c_thread_register() } != 1 {
                            eprintln!("caml_c_thread_register() failed!");
                            abort()
                        }
                    })
                    .on_thread_stop(|| {
                        if unsafe { caml_c_thread_unregister() } != 1 {
                            eprintln!("caml_c_thread_unregister() failed!");
                            abort()
                        }
                    })
                    .build()
                    .unwrap(),
            );
            *weak_rt = Arc::downgrade(&new_rt);
            new_rt
        }
    }
}

ocaml::import! {
    fn olwti_current_executor() -> DynBox<BridgedExecutor>;
}

pub struct LwtExecutorBridge {
    fut: Pin<Box<dyn Future<Output = ()> + Sync + Send + 'static>>,
    waker: Waker,
}

impl LwtExecutorBridge {
    fn new(ex: Arc<Executor<'static>>, notification: Notification) -> Self {
        let waker = waker_fn::waker_fn(move || notification.send());
        Self {
            fut: Box::pin(async move {
                ex.run(futures_lite::future::pending::<()>()).await;
            }),
            waker,
        }
    }

    pub fn tick(&mut self) {
        let mut cx = Context::from_waker(&self.waker);
        let _ = self.fut.as_mut().poll(&mut cx);
    }
}

#[derive(Clone)]
struct ExecutorContext {
    ex: Arc<Executor<'static>>,
}

impl ExecutorContext {
    fn new(ex: Arc<Executor<'static>>) -> Self {
        Self { ex }
    }
}

thread_local! {
    static EXECUTOR_STACK: RefCell<Vec<Rc<ExecutorContext>>> = const { RefCell::new(Vec::new()) };
}

pub struct ExecutorGuard {
    ex_ctx: Rc<ExecutorContext>,
}

impl Drop for ExecutorGuard {
    fn drop(&mut self) {
        EXECUTOR_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            assert!(stack
                .last()
                .map_or(false, |ex_ctx| Rc::ptr_eq(ex_ctx, &self.ex_ctx)));
            stack.pop();
        });
    }
}

pub struct BridgedExecutor {
    pub ex: Arc<Executor<'static>>,
    pub bridge: Mutex<LwtExecutorBridge>,
    pub rt: Arc<tokio::runtime::Runtime>,
}

impl BridgedExecutor {
    pub fn new(notification: Notification) -> BridgedExecutor {
        let ex = Arc::new(Executor::new());
        let bridge = Mutex::new(LwtExecutorBridge::new(ex.clone(), notification));
        let rt = global_tokio_runtime();
        BridgedExecutor { ex, bridge, rt }
    }

    pub fn tick(&self) {
        let mut bridge = self.bridge.lock().unwrap();
        let _guard = self.rt.enter();
        let _self_guard = self.enter();
        bridge.tick();
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.ex.spawn(future)
    }

    pub fn enter(&self) -> ExecutorGuard {
        let ex_ctx = Rc::new(ExecutorContext::new(self.ex.clone()));
        EXECUTOR_STACK.with(|stack| {
            stack.borrow_mut().push(ex_ctx.clone());
        });
        ExecutorGuard { ex_ctx }
    }

    fn current() -> Option<Rc<ExecutorContext>> {
        EXECUTOR_STACK.with(|stack| stack.borrow().last().cloned())
    }
}

pub fn spawn<T>(future: impl Future<Output = T> + Send + 'static) -> Task<T>
where
    T: Send + 'static,
{
    let ctx = BridgedExecutor::current().expect(
        "There is no ocaml-lwt-interop executor context registered for current thread!",
    );
    ctx.ex.spawn(future)
}

pub fn tokio_rt() -> Arc<tokio::runtime::Runtime> {
    global_tokio_runtime()
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
    let _ctx = BridgedExecutor::current().expect(
        "Can't obtain OCaml runtime handle when running outside of ocaml-lwt-interop executor context!",
    );
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
    ctx: ExecutorContext,
}

impl Handle {
    fn new(ctx: ExecutorContext) -> Self {
        Self { ctx }
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.ctx.ex.spawn(future)
    }
}

pub fn handle() -> Handle {
    let ctx = BridgedExecutor::current().expect(
        "There is no ocaml-lwt-interop executor context registered for current thread!",
    );
    Handle::new(ctx.as_ref().clone())
}

pub fn handle_from_runtime(gc: &ocaml::Runtime) -> Handle {
    let bridged_ex = unsafe { olwti_current_executor(gc) }
        .expect("olwti_current_executor has thrown an exception");
    let ctx = ExecutorContext::new(bridged_ex.coerce().ex.clone());
    Handle::new(ctx)
}

pub fn run_with_gc_lock<T: Send>(
    handle: &Handle,
    f: impl FnOnce(&ocaml::Runtime) -> T,
) -> T {
    let (sender, receiver) = mpsc::channel();
    handle
        .spawn(async move {
            yield_now().await;
            unsafe { caml_release_runtime_system() };
            receiver.recv().unwrap();
            unsafe { caml_acquire_runtime_system() };
        })
        .detach();
    unsafe { caml_acquire_runtime_system() };
    let gc = unsafe { ocaml::Runtime::recover_handle() };
    sender.send(()).unwrap();
    let res = f(&gc);
    unsafe { caml_release_runtime_system() };
    res
}
