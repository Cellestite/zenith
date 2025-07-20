use std::any::Any;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::{JoinHandle};
use parking_lot::{Mutex, RwLock};
use crossbeam_queue::SegQueue;
use anyhow::{Result, anyhow};
use zenith_core::collections::{HashMap, SmallVec};
use crate::async_task::{AsyncTask, AsyncTaskHandle, WakerRegistry};
use crate::task::{AsTaskState, BoxedTask, Task, TaskId, TaskResult, TaskState};
use crate::worker::WorkerThread;

pub(crate) type UntypedCompletedFunc = Box<dyn FnOnce(Box<dyn Any + Send + 'static>)>;

pub(crate) struct QueuedTask {
    id: TaskId,
    dependencies: SmallVec<[Arc<TaskState>; 4]>,
}

impl std::fmt::Debug for QueuedTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.id, f)
    }
}

impl QueuedTask {
    fn from(id: TaskId, dependencies: &[Arc<TaskState>]) -> Self {
        Self {
            id,
            dependencies: SmallVec::from(dependencies),
        }
    }

    pub(crate) fn ready_to_execute(&self) -> bool {
        self.dependencies
            .iter()
            .all(|state| state.completed())
    }

    #[inline]
    pub(crate) fn id(&self) -> TaskId {
        self.id
    }
}

#[derive(Debug)]
pub(crate) struct ThreadInfo {
    shutdown: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl ThreadInfo {
    pub(crate) fn new(shutdown: Arc<AtomicBool>, handle: JoinHandle<()>) -> Self {
        Self {
            shutdown,
            handle,
        }
    }

    pub(crate) fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed)
    }

    pub(crate) fn join(self) {
        self.handle.join().unwrap();
    }
}

#[derive(Default)]
pub(crate) struct ThreadLocalState {
    // TODO: replace to Single Consumer Queue, may be user can config whether this queue is a mpsc or spsc queue
    pub(crate) local_queue: SegQueue<QueuedTask>,
    pub(crate) task_storage: Mutex<HashMap<TaskId, BoxedTask>>,
    pub(crate) task_complete_handles: Mutex<HashMap<TaskId, UntypedCompletedFunc>>,
}

pub struct TaskExecutor {
    // TODO: replace to OnceLock
    thread_registry: Arc<RwLock<HashMap<String, ThreadInfo>>>,

    global_queue: Arc<SegQueue<QueuedTask>>,
    thread_local_states: Arc<RwLock<HashMap<String, Arc<ThreadLocalState>>>>,

    task_storage: Arc<Mutex<HashMap<TaskId, BoxedTask>>>,
    task_complete_handles: Arc<Mutex<HashMap<TaskId, UntypedCompletedFunc>>>,

    waker_registry: Arc<WakerRegistry>,
}

unsafe impl Send for TaskExecutor {}
unsafe impl Sync for TaskExecutor {}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new(&[("worker", 8)])
    }
}

impl TaskExecutor {
    pub fn new(thread_configs: &[(&str, usize)]) -> Self {
        let thread_registry = Arc::new(RwLock::new(HashMap::new()));
        let global_queue = Arc::new(SegQueue::new());
        let thread_local_states = Arc::new(RwLock::new(HashMap::new()));
        let task_storage = Arc::new(Mutex::new(HashMap::new()));
        let task_complete_handles = Arc::new(Mutex::new(HashMap::new()));
        let waker_registry = Arc::new(WakerRegistry::new());

        let executor = Self {
            thread_registry,

            global_queue,
            thread_local_states,

            task_storage,
            task_complete_handles,

            waker_registry,
        };
        executor.spawn_threads(thread_configs);
        executor
    }

    pub fn submit<T>(&self, task: T) -> TaskResult<T::Output>
    where
        T: Task + 'static,
        T::Output: Send + 'static,
    {
        let boxed_task = BoxedTask::new(task);
        let task_id = boxed_task.id();

        let task_state = self.register_task(boxed_task, None);
        let handle: TaskResult<T::Output> = TaskResult::from(task_state, task_id);

        self.global_queue.push(QueuedTask::from(task_id, &[]));
        
        handle
    }

    pub fn submit_to<T>(
        &self,
        thread_name: &str,
        task: T,
    ) -> Result<TaskResult<T::Output>>
    where
        T: Task + 'static,
        T::Output: Send + 'static,
    {
        if !self.thread_registry.read().contains_key(thread_name) {
            return Err(anyhow!("Thread '{}' not found", thread_name));
        }

        let boxed_task = BoxedTask::new(task);
        let task_id = boxed_task.id();

        let task_state = self.register_task(boxed_task, Some(thread_name));
        let handle: TaskResult<T::Output> = TaskResult::from(task_state, task_id);

        // directly add to thread's local queue
        {
            let thread_local_states = self.thread_local_states.read();
            if let Some(local_state) = thread_local_states.get(thread_name) {
                local_state.local_queue.push(QueuedTask::from(task_id, &[]));
            } else {
                unreachable!("Try to submit to thread [{}] without registration into TaskExecutor.", thread_name);
            }
        }
        
        Ok(handle)
    }

    pub fn submit_after<T, const N: usize>(
        &self,
        task: T,
        dependencies: [&dyn AsTaskState; N],
    ) -> TaskResult<T::Output>
    where
        T: Task + 'static,
        T::Output: Send + 'static,
    {
        let boxed_task = BoxedTask::new(task);
        let task_id = boxed_task.id();

        let task_state = self.register_task(boxed_task, None);
        let handle: TaskResult<T::Output> = TaskResult::from(task_state, task_id);

        let dependencies = dependencies
            .iter()
            .map(|dependency| dependency.as_state())
            .collect::<SmallVec<[Arc<TaskState>; 4]>>();
        self.global_queue.push(QueuedTask::from(task_id, &dependencies));

        handle
    }

    pub fn submit_to_after<T, const N: usize>(
        &self,
        thread_name: &str,
        task: T,
        dependencies: [&dyn AsTaskState; N],
    ) -> Result<TaskResult<T::Output>>
    where
        T: Task + 'static,
        T::Output: Send + 'static,
    {
        if !self.thread_registry.read().contains_key(thread_name) {
            return Err(anyhow!("Thread '{}' not found", thread_name));
        }

        // TODO: check if all dependencies are in different threads

        let boxed_task = BoxedTask::new(task);
        let task_id = boxed_task.id();

        let task_state = self.register_task(boxed_task, Some(thread_name));
        let handle: TaskResult<T::Output> = TaskResult::from(task_state, task_id);

        // directly add to thread's local queue
        {
            let thread_local_states = self.thread_local_states.read();
            if let Some(local_state) = thread_local_states.get(thread_name) {
                let dependencies = dependencies
                    .iter()
                    .map(|dependency| dependency.as_state())
                    .collect::<SmallVec<[Arc<TaskState>; 4]>>();

                local_state.local_queue.push(QueuedTask::from(task_id, &dependencies));
            } else {
                unreachable!("Try to submit to thread [{}] without registration into TaskExecutor.", thread_name);
            }
        }

        Ok(handle)
    }

    fn register_task(&self, task: BoxedTask, dedicate_thread: Option<&str>) -> Arc<TaskState> {
        let task_id = task.id();
        let task_state = Arc::new(TaskState::new());

        if let Some(thread_name) = dedicate_thread {
            let thread_local_states = self.thread_local_states.read();

            let local_state = thread_local_states
                .get(thread_name)
                .expect(&format!("Try to submit to thread [{}] without registration into TaskExecutor", thread_name));

            local_state.task_storage.lock().insert(task_id, task);
            let inner_task_state = task_state.clone();
            local_state.task_complete_handles.lock().insert(task_id, Box::new(move |result| {
                inner_task_state.set_result(result);
            }));
        } else {
            self.task_storage.lock().insert(task_id, task);
            let inner_task_state = task_state.clone();
            self.task_complete_handles.lock().insert(task_id, Box::new(move |result| {
                inner_task_state.set_result(result);
            }));
        }

        task_state
    }

    pub fn spawn<F>(&self, future: F) -> AsyncTaskHandle<F::Output>
    where
        F: AsyncTask + 'static,
        F::Output: Send + 'static
    {
        let async_task = move || {
            pollster::block_on(future)
        };

        let task = self.submit(async_task);

        let async_handle = AsyncTaskHandle::new(
            task,
            self.waker_registry.clone(),
        );

        async_handle
    }

    pub fn spawn_to<F>(
        &self,
        thread_name: &str,
        future: F,
    ) -> Result<AsyncTaskHandle<F::Output>>
    where
        F: AsyncTask + 'static,
        F::Output: Send + 'static
    {
        let async_task = move || {
            pollster::block_on(future)
        };
        
        let handle = self.submit_to(thread_name, async_task)?;

        let async_handle = AsyncTaskHandle::new(
            handle,
            self.waker_registry.clone(),
        );
        
        Ok(async_handle)
    }

    pub fn spawn_after<F, const N: usize>(
        &self,
        future: F,
        dependencies: [&dyn AsTaskState; N],
    ) -> AsyncTaskHandle<F::Output>
    where
        F: AsyncTask + 'static,
        F::Output: Send + 'static
    {
        let async_task = move || {
            pollster::block_on(future)
        };

        let handle = self.submit_after(async_task, dependencies);

        let async_handle = AsyncTaskHandle::new(
            handle,
            self.waker_registry.clone(),
        );

        async_handle
    }

    pub fn spawn_to_after<F, const N: usize>(
        &self,
        thread_name: &str,
        future: F,
        dependencies: [&dyn AsTaskState; N],
    ) -> Result<AsyncTaskHandle<F::Output>>
    where
        F: AsyncTask + 'static,
        F::Output: Send + 'static
    {
        let async_task = move || {
            pollster::block_on(future)
        };

        let handle = self.submit_to_after(thread_name, async_task, dependencies)?;

        let async_handle = AsyncTaskHandle::new(
            handle,
            self.waker_registry.clone(),
        );

        Ok(async_handle)
    }

    // TODO:
    // pub fn wait_until_idle(&self) {
    //     while !self.global_queue.is_empty() {
    //         std::hint::spin_loop();
    //     }
    //
    //     for thread_local in self.thread_local_states.read().values() {
    //         while !thread_local.local_queue.is_empty() {
    //             std::hint::spin_loop();
    //         }
    //     }
    // }

    pub fn config(&self, thread_configs: &[(&str, usize)]) {
        self.join_all_workers();
        self.spawn_threads(thread_configs);
    }

    pub fn join_all_workers(&self) {
        for (_, thread) in self.thread_registry.write().drain() {
            thread.request_shutdown();
            thread.join();
        }
        self.thread_local_states.write().clear();
        self.waker_registry.clear_all();
    }

    fn spawn_threads(&self, thread_configs: &[(&str, usize)]) {
        for (thread_name, count) in thread_configs {
            for i in 0..(*count as u32) {
                let name = if *count == 1 {
                    (*thread_name).to_owned()
                } else {
                    format!("{}_{}", thread_name, i)
                };

                let shutdown = Arc::new(AtomicBool::new(false));

                let thread_local_state = Arc::new(ThreadLocalState::default());
                self.thread_local_states.write().insert(name.clone(), thread_local_state.clone());

                let worker = WorkerThread::new(
                    shutdown.clone(),

                    self.global_queue.clone(),
                    thread_local_state,

                    self.task_storage.clone(),
                    self.task_complete_handles.clone(),

                    self.waker_registry.clone(),
                );

                let handle = std::thread::Builder::new()
                    .name(name.clone())
                    .spawn(move || worker.run())
                    .expect("Failed to spawn worker thread");

                let info = ThreadInfo::new(shutdown, handle);
                self.thread_registry.write().insert(name, info);
            }
        }
    }
}

impl Drop for TaskExecutor {
    fn drop(&mut self) {
        self.join_all_workers();
    }
}