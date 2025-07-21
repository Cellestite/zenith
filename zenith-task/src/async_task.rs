use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::sync::Arc;
use parking_lot::{RwLock};
use pin_project::pin_project;
use futures::{FutureExt, future::BoxFuture};
use zenith_core::collections::HashMap;
use crate::task::{AsTaskState, TaskId, TaskResult, TaskState};

#[pin_project]
pub struct AsyncTaskHandle<T: Send + 'static> {
    result: TaskResult<T>,
    waker_registry: Arc<WakerRegistry>,
}

impl<T: Send + 'static> AsyncTaskHandle<T> {
    pub(crate) fn new(result: TaskResult<T>, waker_registry: Arc<WakerRegistry>) -> Self {
        Self {
            result,
            waker_registry,
        }
    }

    pub fn null() -> Self {
        Self {
            result: TaskResult::placeholder(),
            waker_registry: Default::default(),
        }
    }

    pub fn wait(&self) {
        self.result.wait();
    }

    pub fn into_blocking(self) -> TaskResult<T> {
        self.result
    }
    
    pub fn completed(&self) -> bool {
        self.result.completed()
    }
}

impl<T: Send + 'static> AsTaskState for AsyncTaskHandle<T> {
    fn as_state(&self) -> &Arc<TaskState> {
        self.result.as_state()
    }
}

impl<T: Send + 'static> Future for AsyncTaskHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let handle = self.project();
        
        if let Some(result) = handle.result.try_into_result() {
            Poll::Ready(result)
        } else {
            handle.waker_registry.register_waker(handle.result.id(), cx.waker().clone());
            Poll::Pending
        }
    }
}

pub trait AsyncTask: IntoFuture + Send {
    fn into_future(self: Box<Self>) -> BoxFuture<'static, Self::Output>;
}

impl<F> AsyncTask for F
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn into_future(self: Box<Self>) -> BoxFuture<'static, Self::Output> {
        (*self).boxed()
    }
}

#[derive(Clone)]
pub(crate) struct WakerRegistry(Arc<RwLock<HashMap<TaskId, Vec<Waker>>>>);

impl WakerRegistry {
    pub(crate) fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub(crate) fn register_waker(&self, task_id: TaskId, waker: Waker) {
        self.0
            .write()
            .entry(task_id)
            .or_insert(Vec::new())
            .push(waker);
    }

    pub(crate) fn wake(&self, task_id: TaskId) {
        let mut wakers = self.0.write();
        if let Some(all_waker) = wakers.remove(&task_id) {
            for waker in all_waker {
                waker.wake();
            }
        }
    }

    pub(crate) fn clear_all(&self) {
        let mut wakers = self.0.write();
        wakers.clear();
    }
}

impl Default for WakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}