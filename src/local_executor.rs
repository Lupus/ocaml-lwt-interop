// Good read on async streams, executors, reactors and tasks:
// https://www.qovery.com/blog/a-guided-tour-of-streams-in-rust

use crate::notification::Notification;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Context,
};

use async_executor::{Executor, Task};

pub struct LwtExecutorBridge {
    fut: Pin<Box<dyn Future<Output = ()> + Sync + Send + 'static>>,
    notification: Notification,
}

impl LwtExecutorBridge {
    // Replace with Arc<Executor<'static>>, or however you're storing the executor
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
        let notification = self.notification.clone();
        let waker = waker_fn::waker_fn(move || notification.send());
        let mut cx = Context::from_waker(&waker);
        let _ = self.fut.as_mut().poll(&mut cx);
    }
}

pub struct BridgedExecutor {
    pub ex: Arc<Executor<'static>>,
    pub bridge: Mutex<LwtExecutorBridge>,
    #[allow(dead_code)]
    notification: Notification,
}

impl BridgedExecutor {
    pub fn new(notification: Notification) -> BridgedExecutor {
        let ex = Arc::new(Executor::new());
        let bridge = Mutex::new(LwtExecutorBridge::new(ex.clone(), notification.clone()));
        BridgedExecutor {
            ex,
            bridge,
            notification,
        }
    }

    pub fn tick(&self) {
        let mut bridge = self.bridge.lock().unwrap();
        bridge.tick();
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        self.ex.spawn(future)
    }
}
