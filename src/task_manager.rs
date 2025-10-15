use tokio::task::JoinHandle;

/// Manages background tasks with proper lifecycle handling
pub struct TaskManager {
    handles: Vec<JoinHandle<()>>,
}

impl TaskManager {
    /// Create a new empty task manager
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    /// Spawn a task and track it
    pub fn spawn<F>(&mut self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        self.handles.push(handle);
    }

    /// Abort all tracked tasks and wait for them to finish
    pub async fn shutdown(self) {
        for handle in self.handles {
            handle.abort();
            // Ignore errors from aborted tasks
            let _ = handle.await;
        }
    }

    /// Abort all tasks without waiting
    pub fn abort_all(&mut self) {
        for handle in &self.handles {
            handle.abort();
        }
        self.handles.clear();
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
