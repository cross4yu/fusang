use std::future::Future;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub type TaskResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct TaskExecutor {
    runtime: Arc<Runtime>,
    sender: mpsc::UnboundedSender<Box<dyn FnOnce() + Send>>,
}

impl TaskExecutor {
    pub fn new() -> Self {
        let runtime = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));

        let (sender, mut receiver) = mpsc::unbounded_channel::<Box<dyn FnOnce() + Send>>();

        runtime.spawn(async move {
            while let Some(task) = receiver.recv().await {
                task();
            }
        });

        Self { runtime, sender }
    }

    pub fn spawn<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    pub fn spawn_blocking<F, T>(&self, func: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.runtime.spawn_blocking(func)
    }

    pub fn send_task<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let _ = self.sender.send(Box::new(task));
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TaskExecutor {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            sender: self.sender.clone(),
        }
    }
}
