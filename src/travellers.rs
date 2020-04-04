use crate::{
    executor::{self, Task},
    ArrayList, Cell, GROUP_SIZE,
};
use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

extern crate quickcheck;

pub trait Traveller<'a> {
    fn setup(&mut self);
    fn traverse(&mut self, workloads: &'a [ArrayList; GROUP_SIZE]) -> u64;
    fn get_name(&self) -> &'static str;
}

pub struct SimpleTraversal;

impl<'a> Traveller<'a> for SimpleTraversal {
    fn traverse(&mut self, workloads: &[ArrayList; GROUP_SIZE]) -> u64 {
        let mut sum: u64 = 0;
        for workload in workloads.iter() {
            let mut pre_idx = 0;
            for _i in 0..workload.list.len() {
                let value = workload.list[pre_idx].get();
                pre_idx = value as usize;
                sum += value;
            }
        }
        sum
    }
    fn get_name(&self) -> &'static str {
        "SimpleTraversal"
    }
    fn setup(&mut self) {}
}

pub struct AsyncTraversal<'a> {
    executor: executor::Executor<'a>,
}

impl<'a> Traveller<'a> for AsyncTraversal<'a> {
    fn setup(&mut self) {}

    fn traverse(&mut self, workloads: &'a [ArrayList; GROUP_SIZE]) -> u64 {
        for workload in workloads.iter() {
            self.executor
                .spawn(Task::new(AsyncTraversal::traverse_one(workload)));
        }
        self.executor.run_ready_task()
    }

    fn get_name(&self) -> &'static str {
        "AsyncTraversal"
    }
}

impl<'a> AsyncTraversal<'a> {
    pub fn new() -> Self {
        AsyncTraversal {
            executor: executor::Executor::new(),
        }
    }

    async fn traverse_one(workload: &ArrayList) -> u64 {
        let mut pre_idx: usize = 0;
        let mut sum: u64 = 0;

        for _i in 0..workload.list.len() {
            unsafe {
                _mm_prefetch(
                    &workload.list[pre_idx] as *const Cell as *const i8,
                    _MM_HINT_T0,
                );
            }
            MemoryAccessFuture::new().await;
            let value = workload.list[pre_idx].get();
            pre_idx = value as usize;
            sum += value;
        }
        sum
    }
}

pub struct MemoryAccessFuture {
    is_first_poll: bool,
}

impl MemoryAccessFuture {
    pub fn new() -> Self {
        MemoryAccessFuture {
            is_first_poll: true,
        }
    }
}

impl Future for MemoryAccessFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_first_poll {
            self.is_first_poll = false;
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ArrayList, AsyncTraversal, SimpleTraversal, Traveller};

    #[quickcheck]
    fn simple_traversal_is_correct(array_size: u8) -> bool {
        if array_size == 0 {
            return true;
        }
        let workloads = [
            ArrayList::new(array_size as usize),
            ArrayList::new(array_size as usize),
            ArrayList::new(array_size as usize),
            ArrayList::new(array_size as usize),
        ];
        let workload_sum = {
            let mut total_sum = 0;
            for workload in workloads.iter() {
                total_sum += workload.ground_truth_sum();
            }
            total_sum
        };

        let mut traveller = SimpleTraversal {};
        let sum = traveller.traverse(&workloads);

        sum == workload_sum
    }

    #[quickcheck]
    fn async_traversal_yield_same_as_simple(array_size: u8) -> bool {
        if array_size == 0 {
            return true;
        }
        let workloads = [
            ArrayList::new(array_size as usize),
            ArrayList::new(array_size as usize),
            ArrayList::new(array_size as usize),
            ArrayList::new(array_size as usize),
        ];

        let mut sync_traveller = SimpleTraversal {};
        let sync_sum = sync_traveller.traverse(&workloads);

        let mut async_traveller = AsyncTraversal::new();
        let async_sum = async_traveller.traverse(&workloads);

        sync_sum == async_sum
    }
}
