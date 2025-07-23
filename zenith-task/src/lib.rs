//! TODO:
//! 1. Reduce global queue contention by taking tasks from global queue and execute them on local queue.
//! 2. If 1 is NOT true, local queue can be removed for some worker threads.
//! 3. Robust result getter (TaskFuture)

mod task;
mod executor;
mod async_task;
mod worker;

use std::future::{IntoFuture};
use std::sync::LazyLock;
use crate::async_task::{AsyncTask, AsyncTaskHandle};
use crate::executor::TaskExecutor;
use crate::task::{AsTaskState, Task};
pub use task::{TaskId, TaskResult, TaskHandle};

static UNIVERSAL_EXECUTOR: LazyLock<TaskExecutor> = LazyLock::new(|| {
    TaskExecutor::default()
});

pub fn submit<T>(task: T) -> TaskResult<T::Output>
where
    T: Task + 'static,
    T::Output: Send + 'static,
{
    UNIVERSAL_EXECUTOR.submit(task)
}

pub fn submit_to<T>(thread_name: &str, task: T) -> anyhow::Result<TaskResult<T::Output>>
where
    T: Task + 'static,
    T::Output: Send + 'static,
{
    UNIVERSAL_EXECUTOR.submit_to(thread_name, task)
}

pub fn submit_after<T, const N: usize>(
    task: T,
    dependencies: [&dyn AsTaskState; N]
) -> TaskResult<T::Output>
where
    T: Task + 'static,
    T::Output: Send + 'static,
{
    UNIVERSAL_EXECUTOR.submit_after(task, dependencies)
}

pub fn submit_to_after<T, const N: usize>(
    thread_name: &str,
    task: T,
    dependencies: [&dyn AsTaskState; N],
) -> anyhow::Result<TaskResult<T::Output>>
where
    T: Task + 'static,
    T::Output: Send + 'static,
{
    UNIVERSAL_EXECUTOR.submit_to_after(thread_name, task, dependencies)
}

pub fn block_on<F: IntoFuture>(future: F) -> F::Output {
    // TODO: true async executor, this will block the thread in thread pool
    pollster::block_on(future)
}

pub fn spawn<F>(future: F) -> AsyncTaskHandle<F::Output>
where
    F: AsyncTask + 'static,
    F::Output: Send + 'static,
{
    UNIVERSAL_EXECUTOR.spawn(future)
}

pub fn spawn_to<F>(
    thread_name: &str,
    future: F,
) -> anyhow::Result<AsyncTaskHandle<F::Output>>
where
    F: AsyncTask + 'static,
    F::Output: Send + 'static,
{
    UNIVERSAL_EXECUTOR.spawn_to(thread_name, future)
}

pub fn spawn_after<F, const N: usize>(
    future: F,
    dependencies: [&dyn AsTaskState; N],
) -> AsyncTaskHandle<F::Output>
where
    F: AsyncTask + 'static,
    F::Output: Send + 'static
{
    UNIVERSAL_EXECUTOR.spawn_after(future, dependencies)
}

pub fn spawn_to_after<F, const N: usize>(
    thread_name: &str,
    future: F,
    dependencies: [&dyn AsTaskState; N],
) -> anyhow::Result<AsyncTaskHandle<F::Output>>
where
    F: AsyncTask + 'static,
    F::Output: Send + 'static,
{
    UNIVERSAL_EXECUTOR.spawn_to_after(thread_name, future, dependencies)
}

pub fn config(thread_configs: &[(&str, usize)]) {
    UNIVERSAL_EXECUTOR.config(thread_configs);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use async_std::task::sleep;
    use parking_lot::Mutex;
    use super::*;

    #[test]
    fn run_tests() {
        println!("Start running tests...\n");

        test_basic_task_execution();
        test_concurrent_task_execution();
        test_task_executor_builder();
        test_cpu_intensive_tasks();

        test_task_with_return_value();
        test_concurrent_tasks_with_return_values();
        test_async_task_submission();
        test_string_processing_tasks();

        test_dependencies();
        test_ring_loop();

        println!("\nAll tests completed！");
    }

    fn calculation_task(val: i32, results_clone: Arc<Mutex<Vec<i32>>>) -> i32 {
        let result = val * val;
        println!("Task {} with calculation result: {}", val, result);
        results_clone.lock().push(result);
        result
    }

    fn test_basic_task_execution() {
        println!("=== test_basic_task_execution() ===");

        let results = Arc::new(Mutex::new(Vec::new()));
        for i in 0..5 {
            let results_clone = Arc::clone(&results);
            submit(move || calculation_task(i, results_clone));
        }

        std::thread::sleep(Duration::from_millis(200));

        let final_results = Mutex::into_inner(Arc::into_inner(results).unwrap());
        println!("All tasks completed, with result: {:?}", final_results);
    }

    fn test_concurrent_task_execution() {
        println!("\n=== test_concurrent_task_execution() ===");

        let num_tasks = 100000i64;

        let start_time = Instant::now();

        let handles = (0..num_tasks)
            .into_iter()
            .map(|i| {
                submit(move || {
                    let mut sum = 0;
                    for _ in 0..10 {
                        sum += i;
                    }

                    sum
                })
            })
            .collect::<Vec<_>>();

        println!("Register {} tasks", num_tasks);

        let result = handles
            .into_iter()
            .map(|handle| handle.get_result())
            .fold(0, |acc, val| acc + val);

        let duration = start_time.elapsed();
        println!("Executed {} tasks, time elapsed: {:?}", num_tasks, duration);
        println!("Final counter: {}", result);

        assert_eq!(result, 49999500000);
    }

    fn test_task_executor_builder() {
        println!("\n=== test_task_executor_builder() ===");

        let message = Arc::new(Mutex::new(String::new()));

        let words = vec!["Hello", "World", "from", "TaskExecutor"];
        for word in words {
            let message_clone = Arc::clone(&message);
            submit(move || {
                let mut msg = message_clone.lock();
                if !msg.is_empty() {
                    msg.push(' ');
                }
                msg.push_str(word);
            });
        }

        std::thread::sleep(Duration::from_millis(200));

        let final_message = Mutex::into_inner(Arc::into_inner(message).unwrap());
        println!("Concat result: {}", final_message);

        assert!(!final_message.is_empty());
    }

    fn test_cpu_intensive_tasks() {
        println!("\n=== test_cpu_intensive_tasks() ===");

        let results = Arc::new(Mutex::new(Vec::new()));

        let fib_inputs = vec![35, 36, 37, 38, 39];

        fn fibonacci(n: u32) -> u64 {
            if n <= 1 {
                n as u64
            } else {
                fibonacci(n - 1) + fibonacci(n - 2)
            }
        }

        let start_time = Instant::now();

        let handles = fib_inputs
            .into_iter()
            .map(|input| {
                let results_clone = Arc::clone(&results);
                submit(move || {
                    let result = fibonacci(input);
                    println!("fibonacci({}) = {}", input, result);
                    results_clone.lock().push((input, result));
                    result
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            handle.wait();
        }
        let duration = start_time.elapsed();
        println!("Fibonacci completed，time elapsed: {:?}", duration);

        let final_results = Mutex::into_inner(Arc::into_inner(results).unwrap());
        println!("Num result: {}", final_results.len());

        for (input, result) in final_results.iter() {
            println!("fib({}) = {}", input, result);
        }

        assert_eq!(final_results.len(), 5);
        assert_eq!(final_results, [(35, 9227465), (36, 14930352), (37, 24157817), (38, 39088169), (39, 63245986)]);
    }

    fn test_task_with_return_value() {
        println!("\n=== test_task_with_return_value() ===");

        let handle = submit(|| {
            let result = 42 * 2;
            println!("Calculating...");
            std::thread::sleep(Duration::from_millis(100));
            result
        });

        let result = handle.get_result();
        println!("Task finished: {}", result);

        assert_eq!(result, 84);
    }

    fn test_concurrent_tasks_with_return_values() {
        println!("\n=== test_concurrent_tasks_with_return_values() ===");

        let mut handles = Vec::new();

        for i in 0..5 {
            let handle = submit(move || {
                let result = i * i;
                println!("Task {} calculation: {} * {} = {}", i, i, i, result);
                result
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            let result = handle.get_result();
            results.push(result);
        }

        println!("All tasks finished: {:?}", results);

        assert_eq!(results.len(), 5);
        assert_eq!(results, [0, 1, 4, 9, 16]);
    }

    fn test_async_task_submission() {
        println!("\n=== test_async_task_submission() ===");

        let future_handle = spawn(async {
            let first_task = async {
                println!("Start a async task");
                println!("In different thread!: {:?}", std::thread::current().name());
                sleep(Duration::from_millis(50)).await;
                "Async result"
            };

            let async_task = spawn(async {
                println!("In different thread!: {:?}", std::thread::current().name());
                sleep(Duration::from_millis(10)).await;
            });
            let async_result = spawn_after(async {
                println!("In different thread!: {:?}", std::thread::current().name());
                sleep(Duration::from_millis(120)).await;
                "Another async result"
            }, [&async_task]);

            let block = async {
                println!("Hello~");
                let mut final_message = first_task.await.to_owned();
                final_message.push_str(async_result.await);
                final_message
            };

            let result = block.await;
            println!("Async result is: {}", result);

            assert_eq!(result, "Async resultAnother async result");
        });
        future_handle.wait();
    }

    fn test_string_processing_tasks() {
        println!("\n=== test_string_processing_tasks() ===");

        let mut handles = Vec::new();

        let words = vec!["Hello", "World", "from", "TaskExecutor", "System"];

        for word in words {
            let handle = submit(move || {
                let processed = format!("{}({})", word, word.len());
                println!("Processing: {} -> {}", word, processed);
                processed
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.get_result());
        }

        println!("Result: {:?}", results);

        assert_eq!(results.len(), 5);
        assert_eq!(results, ["Hello(5)", "World(5)", "from(4)", "TaskExecutor(12)", "System(6)"]);
    }

    fn test_dependencies() {
        println!("\n=== test_dependencies() ===");

        let results = Arc::new(Mutex::new(Vec::new()));

        let inner_results = results.clone();
        let first = submit(move || {
            inner_results.lock().push("first");
            println!("executing thread: {:?}", std::thread::current().name())
        }).into_handle();
        let inner_results = results.clone();
        let second = submit(move || {
            inner_results.lock().push("second");
            println!("executing thread: {:?}", std::thread::current().name())
        });
        let inner_results = results.clone();
        let third = submit_after(move || {
            inner_results.lock().push("third");
            println!("executing thread: {:?}", std::thread::current().name())
        }, [&first, &second]);
        let inner_results = results.clone();
        let fourth = submit_after(move || {
            inner_results.lock().push("fourth");
            println!("executing thread: {:?}", std::thread::current().name())
        }, [&third]);
        let inner_results = results.clone();
        let fifth = submit_after(move || {
            inner_results.lock().push("fifth");
            println!("executing thread: {:?}", std::thread::current().name())
        }, [&third]);
        let inner_results = results.clone();
        let sixth = submit_after(move || {
            inner_results.lock().push("sixth");
            println!("executing thread: {:?}", std::thread::current().name())
        }, [&first]);

        fifth.wait();
        fourth.wait();
        sixth.wait();

        for result in Mutex::into_inner(Arc::into_inner(results).unwrap()) {
            println!("result: {result}");
        }
    }

    fn test_ring_loop() {
        println!("\n=== test_ring_loop() ===");

        config(&[
            ("main", 1),
            ("render", 1),
            ("worker", 2)
        ]);

        let mut start = TaskResult::<()>::placeholder();

        for time in 0..5 {
            let main = submit_to_after("main", move || {
                std::thread::sleep(Duration::from_millis(30 + time * 10));
                println!("Main thread executed!");
            }, [&start]).unwrap();

            let render = submit_to_after("render", move || {
                std::thread::sleep(Duration::from_millis(200 - time * 10));
                println!("Render thread executed!");
            }, [&main]).unwrap();

            start = render;
        }

        start.wait();
    }
}