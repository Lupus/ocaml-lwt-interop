//! This module provides an executor for OCaml domains, integrating Rust async
//! executors with the OCaml runtime.
//!
//! # Overview
//!
//! The `DomainExecutor` struct encapsulates an asynchronous executor designed
//! to be used within an OCaml domain.  It manages the execution of async tasks,
//! ensuring proper interaction with the OCaml runtime system.
//!
//! The executor leverages the [`async_executor`] crate to run async tasks and
//! integrates with the [`tokio`] runtime for asynchronous I/O operations. It
//! provides mechanisms to spawn async tasks within the context of an OCaml
//! domain, ensuring that the OCaml runtime system's domain lock is properly
//! managed during the execution of these tasks.
//!
//! # Usage
//!
//! The `DomainExecutor` can be used to spawn async tasks that need to interact
//! with the OCaml runtime.  It provides functions to enter the executor
//! context, spawn tasks, and ensure that the OCaml runtime lock is
//! appropriately acquired and released when necessary.
//!
//! # Implementation Details
//!
//! The executor stack is managed using a thread-local variable, allowing the
//! executor context to be available within the current thread. The
//! `ExecutorGuard` struct ensures that the executor context is properly managed
//! during execution.
//!
//! The `DomainExecutorDriver` struct drives the executor by polling its future
//! and ensures that tasks are executed in the context of the OCaml domain.
//!
//! The module also provides functions to obtain the current Tokio runtime and
//! to execute code within the OCaml domain lock.
//!

// Good read on async streams, executors, reactors and tasks:
// https://www.qovery.com/blog/a-guided-tour-of-streams-in-rust

use crate::{caml_runtime, notification::Notification};
use std::{
    cell::RefCell,
    future::Future,
    marker::PhantomData,
    panic::UnwindSafe,
    pin::Pin,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex, OnceLock, Weak,
    },
    task::{Context, Waker},
};

use async_executor::{Executor, Task};
use tokio::runtime::Builder;

use ocaml_rs_smartptr::ptr::DynBox;

/// Returns a reference to a global Tokio runtime. While it's global at
/// application level, it's important to note that there might be other Tokio
/// runtimes running along with this one, and "global" refers only to [`crate`]
/// scope and only in a sense like `[#tokio::main]` is creating a global default
/// runtime to be used by the application.
///
/// This function initializes a Tokio runtime in a thread-safe manner, ensuring
/// that only one instance exists throughout the application. The runtime is
/// configured to run in a multi-threaded environment with all features enabled.
///
/// Each worker thread in the runtime is configured to:
/// - Register with the OCaml runtime system upon starting.
/// - Unregister from the OCaml runtime system upon stopping.
///
/// So it it safe to run OCaml code within Tokio tasks on this runtime, if OCaml
/// domain lock is properly acquired.
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
                        format!("olwti-tokio-{}", id)
                    })
                    .on_thread_start(caml_runtime::register_thread)
                    .on_thread_stop(caml_runtime::unregister_thread)
                    .build()
                    .unwrap(),
            );
            *weak_rt = Arc::downgrade(&new_rt);
            new_rt
        }
    }
}

// OCaml callbacks are registered in ../lib/Rust_async.ml
ocaml::import! {
    // `olwti_current_executor` returns the current domain's executor. This
    // function is used to get the executer whenever OCaml runtime handle is
    // available, and helps to avoid passing the executor through whole call
    // chain. It is guaranteed that a single executor is associated to any given
    // OCaml domain.
    fn olwti_current_executor() -> DynBox<DomainExecutor>;
}

/// A driver for the `DomainExecutor`, responsible for running the executor's
/// tasks.
///
/// The `DomainExecutorDriver` encapsulates a future that runs the executor, and
/// a waker that is used to notify the executor when new tasks are available.
///
/// It is designed to be polled periodically to drive the execution of tasks
/// within the executor.
pub struct DomainExecutorDriver {
    fut: Pin<Box<dyn Future<Output = ()> + Sync + Send + 'static>>,
    waker: Waker,
}

impl DomainExecutorDriver {
    /// Creates a new `DomainExecutorDriver` for the given executor.
    ///
    /// The `notification` is used to create a waker that notifies Lwt event
    /// loop when new tasks are available.
    fn new(ex: Arc<Executor<'static>>, notification: Notification) -> Self {
        let waker = waker_fn::waker_fn(move || notification.send());
        Self {
            fut: Box::pin(async move {
                ex.run(futures_lite::future::pending::<()>()).await;
            }),
            waker,
        }
    }

    /// Ticks the executor, polling its future to drive task execution.
    ///
    /// This should be called whenever the notification fires to ensure that
    /// tasks are executed.
    /// Does not block to wait for new tasks, but greedily executes whatever
    /// tasks are available. Some care needs to be taken so as not to starve the
    /// OCaml event loop if large number of tasks are getting spawned/executed.
    pub fn tick(&mut self) {
        let mut cx = Context::from_waker(&self.waker);
        let _ = self.fut.as_mut().poll(&mut cx);
    }
}

/// Context for the `DomainExecutor`, stored in a thread-local stack to manage
/// executor instances.
///
/// The `DomainExecutorContext` holds a reference to the executor and allows the
/// executor to be accessed within the current thread.
#[derive(Clone)]
struct DomainExecutorContext {
    executor: Arc<Executor<'static>>,
}

impl DomainExecutorContext {
    /// Creates a new `DomainExecutorContext` with the given executor.
    fn new(executor: Arc<Executor<'static>>) -> Self {
        Self { executor }
    }
}

thread_local! {
    /// Thread-local stack of `DomainExecutorContext`s, allowing nested executors.
    ///
    /// This stack is used to manage the current executor context within a thread.
    /// When entering a new executor context, it is pushed onto the stack, and
    /// when exiting, it is popped off.
    static EXECUTOR_STACK: RefCell<Vec<Rc<DomainExecutorContext>>> = const { RefCell::new(Vec::new()) };
}

/// A guard that manages the lifetime of an executor context within a scope.
///
/// When an `ExecutorGuard` is created, it pushes the executor context onto the
/// thread-local stack.  When it is dropped, it ensures that the context is
/// popped off, maintaining the correct nesting.
///
/// This is typically used with the `enter` method of `DomainExecutor` to ensure
/// that the executor context is properly set up and torn down.
pub struct ExecutorGuard {
    executor_context: Rc<DomainExecutorContext>,
}

impl Drop for ExecutorGuard {
    /// Drops the guard, removing the executor context from the thread-local
    /// stack.
    fn drop(&mut self) {
        EXECUTOR_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            assert!(stack
                .last()
                .map_or(false, |ex_ctx| Rc::ptr_eq(ex_ctx, &self.executor_context)));
            stack.pop();
        });
    }
}

/// An executor designed to run within an OCaml domain.
///
/// The `DomainExecutor` encapsulates an `async_executor::Executor`, a driver to
/// run it, and a reference to a Tokio runtime.
///
/// It provides methods to spawn tasks, tick the executor, and enter the
/// executor context.
pub struct DomainExecutor {
    /// The async executor instance.
    pub executor: Arc<Executor<'static>>,
    /// The driver that runs the executor.
    pub driver: Mutex<DomainExecutorDriver>,
    /// The Tokio runtime used for asynchronous I/O.
    pub runtime: Arc<tokio::runtime::Runtime>,
}

impl DomainExecutor {
    /// Creates a new `DomainExecutor` with the given notification.
    ///
    /// The `notification` is used to create a waker that notifies when new
    /// tasks are available.
    pub fn new(notification: Notification) -> DomainExecutor {
        let executor = Arc::new(Executor::new());
        let driver =
            Mutex::new(DomainExecutorDriver::new(executor.clone(), notification));
        let runtime = global_tokio_runtime();
        DomainExecutor {
            executor,
            driver,
            runtime,
        }
    }

    /// Ticks the executor, driving task execution.
    ///
    /// This method should be called whenever the notification fires to ensure
    /// that tasks within the executor are executed.
    /// It acquires the executor's driver, enters the Tokio runtime context, enters the executor context,
    /// and then ticks the driver. This ensures that futures being polled have
    /// access to Tokio contenxt and Domain executor context, and are free to
    /// use corresponding API calls, like [`tokio::spawn`] or
    /// [`crate::domain_executor::spawn`].
    pub fn tick(&self) {
        let mut bridge = self.driver.lock().unwrap();
        let _guard = self.runtime.enter();
        let _self_guard = self.enter();
        bridge.tick();
    }

    /// Spawns a new future onto the executor.
    ///
    /// The future must be `Send` and `'static`. Returns a `Task` that can be
    /// used to await the result.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.executor.spawn(future)
    }

    /// Enters the executor context, pushing it onto the thread-local stack.
    ///
    /// Returns an `ExecutorGuard` that will pop the context off the stack when
    /// dropped.
    pub fn enter(&self) -> ExecutorGuard {
        let executor_context = Rc::new(DomainExecutorContext::new(self.executor.clone()));
        EXECUTOR_STACK.with(|stack| {
            stack.borrow_mut().push(executor_context.clone());
        });
        ExecutorGuard { executor_context }
    }

    /// Returns the current `DomainExecutorContext` from the thread-local stack,
    /// if any.
    fn current() -> Option<Rc<DomainExecutorContext>> {
        EXECUTOR_STACK.with(|stack| stack.borrow().last().cloned())
    }
}

/// Spawns a future onto the current executor.
///
/// This function requires that there is an executor context registered in the
/// current thread.  If there is no executor context, it panics.
///
/// # Panics
///
/// Panics if there is no executor context registered in the current thread.
pub fn spawn<T>(future: impl Future<Output = T> + Send + 'static) -> Task<T>
where
    T: Send + 'static,
{
    let ctx = DomainExecutor::current().expect(
        "There is no ocaml-lwt-interop executor context registered for current thread!",
    );
    ctx.executor.spawn(future)
}

/// Returns a reference to the global Tokio runtime.
///
/// This runtime is initialized once and shared across the application. It is
/// safe to call [`crate::domain_executor::run_in_ocaml_domain`] in tasks,
/// spawned on this Tokio runtime, as worker threads are registered to OCaml
/// runtime.
pub fn tokio_rt() -> Arc<tokio::runtime::Runtime> {
    global_tokio_runtime()
}

/// A guard that provides access to the OCaml runtime handle within the current
/// thread.
///
/// This guard ensures that the OCaml runtime handle is only accessed when the
/// executor context is registered in the current thread.
///
/// It implements `Deref` to `ocaml::Runtime`, allowing convenient access to the
/// runtime.
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

/// Obtains a guard to access the OCaml runtime handle.
///
/// This function checks that there is an executor context registered in the
/// current thread, and returns an `OcamlRuntimeGuard` that allows access to the
/// OCaml runtime.
///
/// # Panics
///
/// Panics if there is no executor context registered in the current thread.
pub fn ocaml_runtime<'a>() -> OcamlRuntimeGuard<'a> {
    /* Ensure we're running in a task which is driven by our executor (which is
     * in turn `tick`-ed only from the same OCaml domain) */
    let _ctx = DomainExecutor::current().expect(
        "Can't obtain OCaml runtime handle when running outside of ocaml-lwt-interop executor context!",
    );
    OcamlRuntimeGuard {
        _marker: PhantomData,
        _marker2: PhantomData,
    }
}

/// Spawns a future onto the executor obtained from the OCaml runtime.
///
/// This function is useful if you have synchronous stub function that needs to
/// start some background computation.
pub fn spawn_with_runtime<T>(
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

/// A handle to the OCaml domain executor, allowing tasks to be spawned.
///
/// The `Handle` holds a reference to the executor context and provides methods
/// to spawn tasks. It can safely be shared to another threads or cloned. Can be
/// convenient when you have some Tokio task that needs to spawn some task onto
/// OCaml domain executor later.
///
/// Any thread can use a `Handle` to spawn tasks onto OCaml domain executor,
/// even if it's not one of the threads on global Tokio runtime pool (i.e. the
/// thread does not have to be registered with OCaml runtime, as it will not
/// execute any OCaml-related code, it will just spawn a task which will be
/// executed within the OCaml domain).
#[derive(Clone)]
pub struct Handle {
    ctx: DomainExecutorContext,
}

impl Handle {
    /// Creates a new `Handle` with the given executor context.
    fn new(ctx: DomainExecutorContext) -> Self {
        Self { ctx }
    }

    /// Spawns a future onto the executor associated with this handle.
    ///
    /// The future must be `Send` and `'static`. Returns a `Task` that can be
    /// used to await the result.
    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.ctx.executor.spawn(future)
    }
}

/// Returns a handle to the current OCaml Domain executor.
///
/// # Panics
///
/// Panics if there is no executor context registered in the current thread.
pub fn handle() -> Handle {
    let ctx = DomainExecutor::current().expect(
        "There is no ocaml-lwt-interop executor context registered for current thread!",
    );
    Handle::new(ctx.as_ref().clone())
}

/// Returns a handle to the executor obtained from the OCaml runtime.
///
/// This function is useful if you have synchronous stub function that needs to
/// start some background computation, which in turn needs to spawn some
/// OCaml-specific computation via the handle.
pub fn handle_from_runtime(gc: &ocaml::Runtime) -> Handle {
    let domain_executor = unsafe { olwti_current_executor(gc) }
        .expect("olwti_current_executor has thrown an exception");
    let ctx = DomainExecutorContext::new(domain_executor.coerce().executor.clone());
    Handle::new(ctx)
}

/// Runs a closure `f` within the OCaml domain, ensuring the OCaml runtime lock
/// is acquired.
///
/// This function spawns a task onto the executor associated with the given handle, and then
/// uses synchronization to ensure that the closure `f` is executed while the OCaml runtime lock
/// is held by the current thread, while it is being released by the thread,
/// currently running OCaml domain (and OCaml domain executor).
///
/// It is rather slow and better avoided until you absolutely have to call some
/// OCaml code on the other thread and retrieve some result synchronously. If
/// you can await a future, it's awlays better to spawn a task via
/// [`Handle::spawn`] and await it's result there. Tokio provides a nice
/// [`tokio::task::block_in_place`] which might help avoid calling
/// `run_in_ocaml_domain` in some cases.
///
/// The closure `f` receives a reference to the OCaml runtime.
///
/// # Safety
///
/// This is only safe to call on a thread that is registered with OCaml runtime,
/// i.e. from a Tokio task, spawned using global Tokio runtime obtained via
/// [`tokio_rt`].
///
/// # Panics
///
/// Panics if any of the synchronization primitives fail.
pub unsafe fn run_in_ocaml_domain<T: Send>(
    handle: &Handle,
    f: impl FnOnce(&ocaml::Runtime) -> T + UnwindSafe,
) -> T {
    let (sender, receiver) = mpsc::channel();
    // Spawn a task to be executed on OCaml domain executor
    handle
        .spawn(async move {
            // When OCaml domain executor will get tick()'ed by OCaml domain,
            // this task will start getting executed, and it will release the
            // domain lock
            caml_runtime::with_released_lock(|| {
                // After releasing the domain lock, we wait till other thread
                // communicates back that the lock has been obtained
                receiver.recv().unwrap();
                // After that we block on trying to re-acquire the domain lock
            });
            // This task finishes, OCaml domain executor will proceed executing
            // other tasks
        })
        .detach();

    // Block waiting for domain lock to be acquired
    caml_runtime::with_acquired_lock(move |gc| {
        // Notify receiver that we obtained the lock
        sender.send(()).unwrap();
        // Call the closure
        f(gc)
    }) // Lock is released automatically after `f` completes
}
