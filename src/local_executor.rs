//
// Copyright Stjepan Glavina <stjepang@gmail.com>
// Copyright The OCaml Lwt Interop contributors
//
// Apache License

// This thread-local executor implementation is obtained by forking the
// thread-local executor from async-executor crate [1]. async-executor contains
// both multi-threaded executor and thread-local variant, which is built on top
// of multi-threaded one. This implimentation is basically just inlining of
// multi-threaded executor into thread-local one, removing any multi-thread
// synchronization primitive and dropping unused code. Notable addition is
// callback mechanism to notify external event loop when new pending tasks are
// available for running.
//
// [1] https://github.com/smol-rs/async-executor/blob/master/src/lib.rs

// Good read on async streams, executors, reactors and tasks:
// https://www.qovery.com/blog/a-guided-tour-of-streams-in-rust

use std::cell::RefCell;
use std::fmt;
use std::future::Future;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::rc::Rc;
use std::sync::Arc;
use std::task::Waker;

use async_task::Runnable;
use concurrent_queue::ConcurrentQueue;
use slab::Slab;

#[doc(no_inline)]
pub use async_task::Task;

use crate::{borrow, borrow_mut};

/// A thread-local executor.
#[derive(Clone)]
pub struct LocalExecutor {
    /// The executor state.
    state: Rc<State>,
}

impl UnwindSafe for LocalExecutor {}
impl RefUnwindSafe for LocalExecutor {}

impl fmt::Debug for LocalExecutor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug_executor(&self, "LocalExecutor", f)
    }
}

impl LocalExecutor {
    /// Creates a new executor.
    pub fn new() -> LocalExecutor {
        LocalExecutor {
            state: Rc::new(State::new()),
        }
    }

    pub fn set_notifier(&mut self, notifier: impl Notifier + Send + Sync + 'static) {
        self.state.set_notifier(notifier)
    }

    /// Returns `true` if there are no unfinished tasks.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        borrow!(self.state.active).is_empty()
    }

    /// Attempts to run a task if at least one is scheduled.
    ///
    /// Running a scheduled task means simply polling its future once.
    pub fn try_tick(&self) -> bool {
        match self.state.queue.pop() {
            Err(_) => false,
            Ok(runnable) => {
                // Run the task.
                runnable.run();
                true
            }
        }
    }

    /// Spawns a task onto the executor.
    pub fn spawn<T>(&self, future: impl Future<Output = T>) -> Task<T> {
        let mut active = borrow_mut!(self.state.active);

        // Remove the task from the set of active tasks when the future finishes.
        let index = active.vacant_entry().key();
        let state = self.state.clone();
        let future = async move {
            let _guard = CallOnDrop(move || drop(borrow_mut!(state.active).try_remove(index)));
            future.await
        };

        let notifier = borrow!(self.state.notifier).clone();

        // Create the task and register it in the set of active tasks.
        let (runnable, task) = unsafe {
            // We can't use safe `async_task::spawn` as it expects the future to
            // be Send, while our local futures are not Send (we want them to
            // run only on one thread where the executor was created). So we use
            // `async_task::spawn_unchecked`, which is an unsafe function and
            // bypasses the compiler checks. But we want the compiler to enforce
            // the `schedule` callback to be actually Send, because it will be
            // called from the other threads when they will wake up our local
            // futures - so we wrap it with Self::schedule function, which has
            // Send explicitly marked in it's signature for this purpose.
            async_task::spawn_unchecked(future, Self::schedule(&self.state.queue, notifier))
        };
        active.insert(runnable.waker());

        runnable.schedule();
        task
    }

    /// Returns a function that schedules a runnable task when it gets woken up
    /// Returned function has to be Sync + Send, our local futures might get
    /// woken up by other threads
    fn schedule(
        queue: &ConcurrentQueue<Runnable>,
        notifier: Option<Arc<dyn Notifier + Send + Sync>>,
    ) -> impl Fn(Runnable) + Sync + Send + '_ {
        move |runnable| {
            queue.push(runnable).unwrap();
            match &notifier {
                Some(notifier) => notifier.to_owned().notify(),
                None => (),
            };
        }
    }
}

impl Drop for LocalExecutor {
    fn drop(&mut self) {
        let mut active = borrow_mut!(self.state.active);
        for w in active.drain() {
            w.wake();
        }
        drop(active);

        while self.state.queue.pop().is_ok() {}
    }
}

impl Default for LocalExecutor {
    fn default() -> LocalExecutor {
        LocalExecutor::new()
    }
}

/// The state of a executor.
struct State {
    /// The global queue.
    queue: ConcurrentQueue<Runnable>,

    /// Optional notify callback
    notifier: RefCell<Option<Arc<dyn Notifier + Send + Sync>>>,

    /// Currently active tasks.
    active: RefCell<Slab<Waker>>,
}

impl State {
    /// Creates state for a new executor.
    fn new() -> State {
        State {
            queue: ConcurrentQueue::unbounded(),
            notifier: RefCell::new(None),
            active: RefCell::new(Slab::new()),
        }
    }
    fn set_notifier(&self, notifier: impl Notifier + Send + Sync + 'static) {
        let mut self_notifier = borrow_mut!(self.notifier);
        *self_notifier = Some(Arc::new(notifier))
    }
}

pub trait Notifier {
    fn notify(&self);
}

/// Debug implementation for `Executor` and `LocalExecutor`.
fn debug_executor(executor: &LocalExecutor, name: &str, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct(name)
        .field("active", &borrow!(executor.state.active).len())
        .field("global_tasks", &executor.state.queue.len())
        .finish()
}

/// Runs a closure when dropped.
struct CallOnDrop<F: Fn()>(F);

impl<F: Fn()> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}
