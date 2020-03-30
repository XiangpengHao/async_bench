use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use std::default::Default;

const EXECUTOR_QUEUE_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskId(usize);

pub struct Task {
    future: Pin<Box<dyn Future<Output = u64>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = u64> + 'static) -> Task {
        Task {
            future: Box::pin(future),
        }
    }

    pub fn id(&self) -> TaskId {
        use core::ops::Deref;
        let addr = Pin::deref(&self.future) as *const _ as *const () as usize;
        TaskId(addr)
    }

    fn poll(&mut self, context: &mut Context) -> Poll<u64> {
        self.future.as_mut().poll(context)
    }
}

pub struct Executor {
    task_queue: [Option<Task>; EXECUTOR_QUEUE_SIZE],
    next_slot: u16,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            task_queue: Default::default(),
            next_slot: 0,
        }
    }

    pub fn spawn(&mut self, task: Task) {
        if self.next_slot as usize == EXECUTOR_QUEUE_SIZE {
            panic!("max executor queue reached!");
        }
        self.task_queue[self.next_slot as usize] = Some(task);
        self.next_slot += 1;
    }

    pub fn run_ready_task(&mut self) -> u64 {
        let mut pos: u8 = 0;
        let mut ready_task: u8 = 0;
        let mut total_sum: u64 = 0;

        loop {
            if let Some(mut task) = self.task_queue[pos as usize].take() {
                let waker = dummy_waker();
                let mut context = Context::from_waker(&waker);
                match task.poll(&mut context) {
                    Poll::Ready(sum) => {
                        ready_task += 1;
                        total_sum += sum;
                    }
                    Poll::Pending => {
                        self.task_queue[pos as usize] = Some(task);
                    }
                }
            }
            pos += 1;

            if ready_task == 4 {
                return total_sum;
            }

            // TODO: we can avoid this branch
            if pos == 4 {
                pos = 0;
            }
        }
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }
    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

pub struct MemoryAccessFuture {}

impl Future for MemoryAccessFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}
