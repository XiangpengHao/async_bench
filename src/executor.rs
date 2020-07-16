use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use std::mem::{self, MaybeUninit};

const EXECUTOR_QUEUE_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskId(usize);

pub struct Executor<F, const N: usize> {
    task_queue: [Option<F>; N],
}

impl<F: Future, const N: usize> Executor<F, N> {
    pub fn new() -> Self {
        Executor {
            task_queue: [None; N],
        }
    }

    pub fn spawn(&mut self, task: F) {
        for i in 0..EXECUTOR_QUEUE_SIZE {
            if self.task_queue[i].is_none() {
                self.task_queue[i] = Some(task);
                return;
            }
        }
        panic!("max executor queue reached!");
    }

    pub fn run_ready_tasks(&mut self) -> [F::Output; N] {
        let mut pos: usize = 0;
        let mut ready_task: u8 = 0;
        let mut output: [MaybeUninit<F::Output>; N] =
            unsafe { MaybeUninit::uninit().assume_init() };

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);

        loop {
            if let Some(task) = self.task_queue[pos as usize].as_mut() {
                let pinned_task = unsafe { Pin::new_unchecked(task) };
                if let Poll::Ready(sum) = pinned_task.poll(&mut context) {
                    ready_task += 1;
                    output[pos] = MaybeUninit::new(sum);
                    self.task_queue[pos] = None;
                }
            }
            pos += 1;

            if ready_task == N as u8 {
                let ret = unsafe { mem::transmute_copy(&output) };
                mem::forget(output);
                return ret;
            }

            pos = pos % N;
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
