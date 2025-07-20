use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crossbeam_queue::SegQueue;
use parking_lot::{Mutex};
use zenith_core::collections::HashMap;
use crate::executor::{QueuedTask, ThreadLocalState, UntypedCompletedFunc};
use crate::async_task::WakerRegistry;
use crate::task::{BoxedTask, TaskId};

pub(crate) struct WorkerThread {
    shutdown: Arc<AtomicBool>,

    global_queue: Arc<SegQueue<QueuedTask>>,
    local_state: Arc<ThreadLocalState>,

    task_storage: Arc<Mutex<HashMap<TaskId, BoxedTask>>>,
    task_complete_handles: Arc<Mutex<HashMap<TaskId, UntypedCompletedFunc>>>,

    waker_registry: Arc<WakerRegistry>,
}

unsafe impl Send for WorkerThread {}

impl WorkerThread {
    pub(crate) fn new(
        shutdown: Arc<AtomicBool>,

        global_queue: Arc<SegQueue<QueuedTask>>,
        local_state: Arc<ThreadLocalState>,

        task_storage: Arc<Mutex<HashMap<TaskId, BoxedTask>>>,
        task_complete_handles: Arc<Mutex<HashMap<TaskId, UntypedCompletedFunc>>>,

        waker_registry: Arc<WakerRegistry>,
    ) -> Self {
        Self {
            shutdown,

            global_queue,
            local_state,

            task_storage,
            task_complete_handles,

            waker_registry,
        }
    }

    pub(crate) fn run(self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            let mut executed_local_task = false;
            // 1. consume all local tasks (higher priority)
            loop {
                // find next available task (has no dependencies)
                while let Some(task) = self.local_state.local_queue.pop() {
                    if task.ready_to_execute() {
                        executed_local_task = self.execute_local_task(task.id());
                        break;
                    } else {
                        // Not ready, put it back to the global queue
                        self.local_state.local_queue.push(task);
                    }
                }

                break;
            }

            let mut executed_global_task = false;
            // 2. try to steal task from global queue if free from local queue.
            if !executed_local_task {
                // find next available task (has no dependencies)
                loop {
                    if let Some(task) = self.global_queue.pop() {
                        if task.ready_to_execute() {
                            executed_global_task = self.execute_task(task.id());
                            break;
                        } else {
                            // Not ready, put it back to the global queue
                            self.global_queue.push(task);
                        }
                    } else {
                        break;
                    }
                }
            }

            if !executed_local_task && !executed_global_task {
                // no work available, sleep a while
                std::thread::sleep(Duration::from_micros(10));
            }
        }
    }

    fn execute_local_task(&self, task_id: TaskId) -> bool {
        let task = self.local_state.task_storage.lock().remove(&task_id);

        let mut executed_task = false;
        if let Some(task) = task {
            let result = task.execute();

            // notify task handles
            if let Some(completed_fn) = self.local_state.task_complete_handles.lock().remove(&task_id) {
                completed_fn(result);
            }

            // notify futures
            self.waker_registry.wake(task_id);
            executed_task = true;
        }

        executed_task
    }

    fn execute_task(&self, task_id: TaskId) -> bool {
        let task = self.task_storage.lock().remove(&task_id);

        let mut executed_task = false;
        if let Some(task) = task {
            let result = task.execute();

            // notify task handles
            if let Some(completed_fn) = self.task_complete_handles.lock().remove(&task_id) {
                completed_fn(result);
            }

            // notify futures
            self.waker_registry.wake(task_id);
            executed_task = true;
        }

        executed_task
    }
}
