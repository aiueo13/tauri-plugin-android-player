#![cfg(target_os = "android")]

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};


pub struct ScopeGuard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> ScopeGuard<F> {

    pub fn new(on_drop: F) -> Self {
        Self(Some(on_drop))
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {

    fn drop(&mut self) {
        if let Some(on_drop) = self.0.take() {
            (on_drop)();
        }
    }
}

/// Based on code from Tokio crate
///
/// Source:
/// - <https://docs.rs/tokio/1.53.1/src/tokio/runtime/task/error.rs.html>
/// - Copyright (c) Tokio Contributors
/// - Licensed under the MIT License
pub fn panic_payload_as_str(payload: &SyncWrapper<Box<dyn Any + Send>>) -> Option<&str> {
    // Panic payloads are almost always `String` (if invoked with formatting arguments)
    // or `&'static str` (if invoked with a string literal).
    //
    // Non-string panic payloads have niche use-cases,
    // so we don't really need to worry about those.
    if let Some(s) = payload.downcast_ref_sync::<String>() {
        return Some(s);
    }

    if let Some(s) = payload.downcast_ref_sync::<&'static str>() {
        return Some(s);
    }

    None
}

pub use sync_wrapper::SyncWrapper;

/// Based on code from Tokio crate
///
/// Source:
/// - <https://docs.rs/tokio/1.53.1/src/tokio/util/sync_wrapper.rs.html>
/// - Copyright (c) Tokio Contributors
/// - Licensed under the MIT License
#[allow(unused)]
mod sync_wrapper {
    // This module contains a type that can make `Send + !Sync` types `Sync` by
    // disallowing all immutable access to the value.
    //
    // A similar primitive is provided in the `sync_wrapper` crate.

    use std::any::Any;

    pub struct SyncWrapper<T> {
        value: T,
    }

    // safety: The SyncWrapper being send allows you to send the inner value across
    // thread boundaries.
    unsafe impl<T: Send> Send for SyncWrapper<T> {}

    // safety: An immutable reference to a SyncWrapper is useless, so moving such an
    // immutable reference across threads is safe.
    unsafe impl<T> Sync for SyncWrapper<T> {}

    impl<T> SyncWrapper<T> {

        pub(crate) fn new(value: T) -> Self {
            Self { value }
        }

        pub(crate) fn into_inner(self) -> T {
            self.value
        }
    }

    impl SyncWrapper<Box<dyn Any + Send>> {
        /// Attempt to downcast using `Any::downcast_ref()` to a type that is known to be `Sync`.
        pub(crate) fn downcast_ref_sync<T: Any + Sync>(&self) -> Option<&T> {
            // SAFETY: if the downcast fails, the inner value is not touched,
            // so no thread-safety violation can occur.
            self.value.downcast_ref()
        }
    }
}

pub async fn sleep(time: Duration) {
    SLEEP_MANAGER.sleep(time).await;
}

static SLEEP_MANAGER: LazyLock<SleepManager> = LazyLock::new(SleepManager::new);


struct SleepManager {
    inner: Arc<(Mutex<ManagerState>, Condvar)>,
}

struct ManagerState {
    tasks: Vec<SleepTask>,
    thread_running: bool,
}

struct SleepTask {
    deadline: Instant,
    shared_state: Arc<Mutex<SharedTaskState>>,
}

struct SharedTaskState {
    completed: bool,
    waker: Option<Waker>,
}

impl SleepManager {

    fn new() -> Self {
        Self {
            inner: Arc::new((
                Mutex::new(ManagerState {
                    tasks: Vec::new(),
                    thread_running: false,
                }),
                Condvar::new(),
            )),
        }
    }

    fn sleep(&self, time: Duration) -> SleepFuture {
        let shared_task_state = Arc::new(Mutex::new(SharedTaskState {
            completed: false,
            waker: None,
        }));

        let task = SleepTask {
            deadline: Instant::now() + time,
            shared_state: shared_task_state.clone(),
        };

        let (lock, cvar) = &*self.inner;
        let mut state = lock.lock().unwrap();
        
        state.tasks.push(task);

        if !state.thread_running {
            state.thread_running = true;
            let inner_clone = self.inner.clone();

            drop(state);

            tauri::async_runtime::spawn_blocking(move || {
                let (lock, cvar) = &*inner_clone;
                let mut state = lock.lock().unwrap();

                loop {
                    let now = Instant::now();
                    
                    // 期限切れのタスクを処理する
                    state.tasks.retain(|task| {
                        if task.deadline <= now {
                            let mut ts = task.shared_state.lock().unwrap();
                            ts.completed = true;
                            if let Some(waker) = ts.waker.take() {
                                waker.wake();
                            }
                            false
                        } 
                        else {
                            true
                        }
                    });

                    // タスクがなければスレッドを終了する
                    if state.tasks.is_empty() {
                        state.thread_running = false;
                        break;
                    }

                    // 次に期限を迎えるタスクまで待機する
                    let next_deadline = state.tasks.iter().map(|t| t.deadline).min().unwrap();
                    let now = Instant::now();
                    if now < next_deadline {
                        let wait_time = next_deadline - now;
                        let (new_state, _) = cvar.wait_timeout(state, wait_time).unwrap();
                        state = new_state;
                    }
                }
            });
        }
        else {
            drop(state);

            // すでにスレッドが待機中の場合、新しく追加したタスクの期限が
            // 既存のタスクより早い可能性があるので、一度スレッドを起こして再計算させる
            cvar.notify_one();
        }

        SleepFuture { shared_state: shared_task_state }
    }
}

struct SleepFuture {
    shared_state: Arc<Mutex<SharedTaskState>>,
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.shared_state.lock().unwrap();
        
        if state.completed {
            Poll::Ready(())
        }
        else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}