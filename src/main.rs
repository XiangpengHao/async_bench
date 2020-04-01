use crate::executor::Task;
use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use std::time::Instant;
#[macro_use]
extern crate lazy_static;

use structopt::StructOpt;
use workloads::{ArrayList, Cell, ARRAY_SIZE};

pub mod executor;
pub mod workloads;

const GROUP_SIZE: usize = 4;

lazy_static! {
    pub static ref WORKLOADS: [Box<ArrayList>; GROUP_SIZE] = unsafe {
        let mut data: [std::mem::MaybeUninit<Box<ArrayList>>; GROUP_SIZE] =
            std::mem::MaybeUninit::uninit().assume_init();
        for elem in &mut data[..] {
            std::ptr::write(elem.as_mut_ptr(), ArrayList::new());
        }
        std::mem::transmute::<_, [Box<ArrayList>; GROUP_SIZE]>(data)
    };
}
trait Traveller<'a> {
    fn setup(&mut self);
    fn traverse(&mut self, workloads: &'a [Box<ArrayList>; GROUP_SIZE]) -> u64;
    fn get_name(&self) -> &'static str;
}

struct SimpleTraversal;

impl<'a> Traveller<'a> for SimpleTraversal {
    fn traverse(&mut self, workloads: &[Box<ArrayList>; GROUP_SIZE]) -> u64 {
        let mut sum: u64 = 0;
        for workload in workloads.iter() {
            let mut pre_idx = 0;
            for _i in 0..ARRAY_SIZE {
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

struct AsyncTraversal<'a> {
    executor: executor::Executor<'a>,
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

impl<'a> Traveller<'a> for AsyncTraversal<'a> {
    fn setup(&mut self) {}

    fn traverse(&mut self, workloads: &'a [Box<ArrayList>; GROUP_SIZE]) -> u64 {
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
    fn new() -> Self {
        AsyncTraversal {
            executor: executor::Executor::new(),
        }
    }

    async fn traverse_one(workload: &Box<ArrayList>) -> u64 {
        let mut pre_idx: usize = 0;
        let mut sum: u64 = 0;

        for _i in 0..ARRAY_SIZE {
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

fn benchmark<'a>(
    workloads: &'a [Box<ArrayList>; GROUP_SIZE],
    mut traveller: impl Traveller<'a>,
    options: &CommandLineOptions,
) {
    traveller.setup();

    for i in 0..options.repetition {
        let time_begin = Instant::now();
        let sum = traveller.traverse(&workloads);
        let elapsed = time_begin.elapsed().as_nanos();

        println!("{}#{}: {} ns", traveller.get_name(), i, elapsed);
        assert_eq!(sum, WORKLOADS[0].ground_truth_sum() * GROUP_SIZE as u64);
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "async_bench")]
struct CommandLineOptions {
    #[structopt(short, long)]
    traveller: String,

    #[structopt(short, long, default_value = "3")]
    repetition: i32,
}

fn main() {
    let options = CommandLineOptions::from_args();

    let workloads: [Box<ArrayList>; GROUP_SIZE] = unsafe {
        let mut data: [std::mem::MaybeUninit<Box<ArrayList>>; GROUP_SIZE] =
            std::mem::MaybeUninit::uninit().assume_init();
        for elem in &mut data[..] {
            std::ptr::write(elem.as_mut_ptr(), ArrayList::new());
        }
        std::mem::transmute::<_, [Box<ArrayList>; GROUP_SIZE]>(data)
    };

    if options.traveller == "sync" {
        let traveller = SimpleTraversal {};
        benchmark(&workloads, traveller, &options);
    } else if options.traveller == "async" {
        let traveller: AsyncTraversal = AsyncTraversal::new();
        benchmark(&workloads, traveller, &options);
    }
}
