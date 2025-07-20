use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::any::Any;
use std::sync::Arc;
use parking_lot::{Condvar, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    pub const INVALID: TaskId = TaskId(u64::MAX);
    
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        TaskId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn valid(&self) -> bool {
        self != &Self::INVALID
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task#{}", self.0)
    }
}

pub(crate) type UntypedThreadSafeObject = Box<dyn Any + Send + 'static>;
pub(crate) type UntypedExecuteFunc = Box<dyn FnOnce(Box<dyn Any + Send + 'static>) -> Box<dyn Any + Send + 'static>>;


pub trait Task: Send + 'static {
    type Output: Send + 'static;
    
    fn execute(self: Box<Self>) -> Self::Output;
}

impl<F, R> Task for F
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    type Output = R;

    fn execute(self: Box<Self>) -> Self::Output {
        (*self)()
    }
}

pub(crate) struct BoxedTask {
    id: TaskId,
    task: UntypedThreadSafeObject,
    execute_fn: UntypedExecuteFunc,
}

impl BoxedTask {
    pub(crate) fn new<T: Task + 'static>(task: T) -> Self {
        let id = TaskId::new();
        let execute_fn = Box::new(|task_any: UntypedThreadSafeObject| -> UntypedThreadSafeObject {
            let task = task_any.downcast::<T>().expect("Task type mismatch");
            let result = task.execute();
            Box::new(result)
        });

        Self {
            id,
            task: Box::new(task),
            execute_fn,
        }
    }

    pub(crate) fn execute(self) -> Box<dyn Any + Send> {
        (self.execute_fn)(self.task)
    }

    pub(crate) fn id(&self) -> TaskId {
        self.id
    }
}

pub trait AsTaskState {
    fn as_state(&self) -> Arc<TaskState>;
}

pub struct TaskState {
    result: Mutex<Option<UntypedThreadSafeObject>>,
    completed: AtomicBool,
    condvar: Condvar,
}

impl TaskState {
    pub(crate) fn new() -> Self {
        Self {
            result: Mutex::new(None),
            completed: AtomicBool::new(false),
            condvar: Condvar::new(),
        }
    }

    pub(crate) fn set_result(&self, result: UntypedThreadSafeObject) {
        *self.result.lock() = Some(result);
        self.set_completed();
    }

    pub(crate) fn completed(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }

    pub(crate) fn set_completed(&self) {
        self.completed.fetch_or(true, Ordering::AcqRel);
        self.condvar.notify_all();
    }

    pub(crate) fn wait(&self) {
        if self.completed.load(Ordering::Acquire) {
            return;
        }

        let mut result = self.result.lock();
        while !self.completed.load(Ordering::Acquire) {
            self.condvar.wait(&mut result);
        }
    }
}

pub struct TaskResult<T: Send + 'static> {
    id: TaskId,
    state: Arc<TaskState>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Send + 'static> Clone for TaskResult<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            id: self.id,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + Send + 'static> TaskResult<T> {
    pub fn try_get_cloned(&self) -> Option<T>
    where
        T: Send + 'static,
    {
        if self.state.completed.load(Ordering::Acquire) {
            self.state.result.lock().as_ref()?.downcast_ref().cloned()
        } else {
            None
        }
    }

    pub fn get_cloned(&self) -> T
    where
        T: Send + 'static,
    {
        self.wait();

        if self.state.completed.load(Ordering::Acquire) {
            self.state.result.lock()
                .as_ref()
                .expect("Task is not completed or result had been taken!")
                .downcast_ref::<T>()
                .expect("Result type mismatched!")
                .clone()
        } else {
            panic!("Task is not completed!")
        }
    }
}

impl<T: Send + 'static> TaskResult<T> {
    pub fn null() -> Self {
        Self {
            id: TaskId::INVALID,
            state: Arc::new(TaskState {
                result: Default::default(),
                completed: AtomicBool::new(true),
                condvar: Default::default(),
            }),
            _phantom: std::marker::PhantomData,
        }
    }

    pub(crate) fn from(state: Arc<TaskState>, id: TaskId) -> Self {
        Self {
            state,
            id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn completed(&self) -> bool {
        self.state.completed.load(Ordering::Acquire)
    }

    pub fn wait(&self) {
        self.state.wait();
    }

    pub fn try_get(&self) -> Option<T>
    where
        T: Send + 'static,
    {
        if self.state.completed.load(Ordering::Acquire) {
            let mut result = self.state.result.lock();

            if result.is_none() {
                return None;
            }

            result.take()?.downcast().ok().map(|boxed| *boxed)
        } else {
            None
        }
    }

    pub fn get(self) -> T
    where
        T: Send + 'static,
    {
        self.wait();

        if self.state.completed.load(Ordering::Acquire) {
            *self.state.result.lock().take()
                .expect("Task is not completed or result had been taken!")
                .downcast()
                .expect("Result type mismatched!")
        } else {
            panic!("Task is not completed!")
        }
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn forget_result(self) -> TaskHandle {
        TaskHandle {
            id: self.id,
            state: self.state,
        }
    }
}

impl<T: Send + 'static> AsTaskState for TaskResult<T> {
    fn as_state(&self) -> Arc<TaskState> {
        self.state.clone()
    }
}

pub struct TaskHandle {
    id: TaskId,
    state: Arc<TaskState>,
}

impl TaskHandle {
    pub fn null() -> Self {
        Self {
            id: TaskId::INVALID,
            state: Arc::new(TaskState {
                result: Default::default(),
                completed: AtomicBool::new(true),
                condvar: Default::default(),
            }),
        }
    }

    pub fn completed(&self) -> bool {
        self.state.completed.load(Ordering::Acquire)
    }

    pub fn wait(&self) {
        self.state.wait()
    }

    pub fn id(&self) -> TaskId {
        self.id
    }
}

impl AsTaskState for TaskHandle {
    fn as_state(&self) -> Arc<TaskState> {
        self.state.clone()
    }
}
