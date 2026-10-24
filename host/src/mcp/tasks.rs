//! A proxy owns its readers/writers even when a transport error returns early.
#[derive(Default)]
pub(super) struct Tasks(Vec<tokio::task::AbortHandle>);
impl Tasks {
    pub(super) fn track<T>(&mut self, task: &tokio::task::JoinHandle<T>) {
        self.0.push(task.abort_handle());
    }
}
impl Drop for Tasks {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}
