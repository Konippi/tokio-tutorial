use crossbeam::channel;
use futures::{
    future::BoxFuture,
    task::{self, ArcWake},
};
use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    process::exit,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread::{self},
    time::{Duration, Instant},
};

fn main() {
    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async {
        spawn(async {
            delay(Duration::from_millis(100)).await;
            println!("task 1");
        });

        spawn(async {
            delay(Duration::from_millis(200)).await;
            println!("task 2");
        });

        delay(Duration::from_millis(300)).await;
        println!("task done");

        exit(0);
    });

    mini_tokio.run();
}

struct MiniTokio {
    // Receiver for scheduled tasks.
    // When a task is scheduled, the associated future is ready to make progress.
    receiver: channel::Receiver<Arc<Task>>,

    // Sender to schedule tasks from other threads.
    // When a future is ready to make progress, it will schedule itself to this sender.
    sender: channel::Sender<Arc<Task>>,
}

impl MiniTokio {
    /// Create a new MiniTokio instance.
    fn new() -> Self {
        let (sender, receiver) = channel::unbounded();

        Self { receiver, sender }
    }

    /// Spawn a future onto the MiniTokio instance.
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender);
    }

    /// Run the MiniTokio instance to completion.
    fn run(&mut self) {
        CURRENT.with(|cell| {
            *cell.borrow_mut() = Some(self.sender.clone());
        });

        while let Ok(task) = self.receiver.recv() {
            task.poll();
        }
    }
}

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    CURRENT.with(|cell| {
        let borrow = cell.borrow();
        let sender = borrow.as_ref().unwrap();
        Task::spawn(future, sender);
    });
}

thread_local! {
    static CURRENT: RefCell<Option<channel::Sender<Arc<Task>>>> = RefCell::new(None);
}

struct Task {
    future: Mutex<BoxFuture<'static, ()>>,
    executor: channel::Sender<Arc<Task>>,
}

impl Task {
    fn spawn<F>(future: F, sender: &channel::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });
        let _ = sender.send(task);
    }

    fn poll(self: Arc<Self>) {
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);
        let mut future = self.future.try_lock().unwrap();
        let _ = future.as_mut().poll(&mut cx);
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let _ = arc_self.executor.send(arc_self.clone());
    }
}

async fn delay(duration: Duration) {
    struct Delay {
        when: Instant,
        waker: Option<Arc<Mutex<Waker>>>,
    }

    impl Future for Delay {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if let Some(waker) = &self.waker {
                let mut waker = waker.lock().unwrap();

                if !waker.will_wake(cx.waker()) {
                    *waker = cx.waker().clone();
                }
            } else {
                let when = self.when;
                let waker = Arc::new(Mutex::new(cx.waker().clone()));
                self.waker = Some(waker.clone());

                thread::spawn(move || {
                    let now = Instant::now();

                    if now < when {
                        thread::sleep(when - now);
                    }

                    let waker = waker.lock().unwrap();
                    waker.wake_by_ref();
                });
            }

            if Instant::now() >= self.when {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }

    let future = Delay {
        when: Instant::now() + duration,
        waker: None,
    };

    future.await;
}
