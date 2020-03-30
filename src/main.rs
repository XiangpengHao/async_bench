use crate::executor::{MemoryAccessFuture, Task};
use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::alloc::{alloc, Layout};
use std::mem;
use std::time::Instant;
#[macro_use]
extern crate lazy_static;

use structopt::StructOpt;

pub mod executor;

const ARRAY_SIZE: usize = 1024 * 1024;
const GROUP_SIZE: usize = 4;

trait Traveller {
    fn setup(&mut self);
    fn traverse(&mut self, workloads: &'static [Box<ArrayList>; GROUP_SIZE]) -> u64;
    fn get_name(&self) -> &'static str;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Cell {
    next_index: u64,
    _padding: [u64; 7],
}

impl Cell {
    fn set(&mut self, value: u64) {
        self.next_index = value;
    }
    fn get(&self) -> u64 {
        self.next_index
    }
}

lazy_static! {
    static ref WORKLOADS: [Box<ArrayList>; GROUP_SIZE] = unsafe {
        let mut data: [std::mem::MaybeUninit<Box<ArrayList>>; GROUP_SIZE] =
            std::mem::MaybeUninit::uninit().assume_init();
        for elem in &mut data[..] {
            std::ptr::write(elem.as_mut_ptr(), ArrayList::new());
        }
        std::mem::transmute::<_, [Box<ArrayList>; GROUP_SIZE]>(data)
    };
}

#[repr(C)]
struct ArrayList {
    list: [Cell; ARRAY_SIZE],
}

impl ArrayList {
    fn new() -> Box<Self> {
        let mut workload_list = ArrayList::create_array();
        let mut temp_values: Vec<u64> = Vec::with_capacity(ARRAY_SIZE - 1);
        for i in 1..ARRAY_SIZE {
            temp_values.push(i as u64);
        }
        temp_values.shuffle(&mut thread_rng());

        let mut pre_idx = 0;
        for elem in temp_values.iter() {
            workload_list.list[pre_idx].set(*elem);
            pre_idx = *elem as usize;
        }
        workload_list
    }

    /// We can't simply Box::new([0; 3000000]); because it will overflow the stack
    /// https://github.com/rust-lang/rust/issues/53827
    fn create_array() -> Box<ArrayList> {
        let layout =
            Layout::from_size_align(mem::size_of::<Cell>() * ARRAY_SIZE, mem::align_of::<Cell>())
                .expect("should success");
        let array_list = unsafe {
            let ptr = alloc(layout) as *mut ArrayList;
            Box::from_raw(ptr)
        };
        array_list
    }

    const fn ground_truth_sum(&self) -> u64 {
        ((0 + ARRAY_SIZE - 1) * ARRAY_SIZE / 2) as u64
    }

    fn _print_values(&self) {
        for elem in self.list.iter() {
            print!("{}\t", elem.next_index);
        }
        println!("");
    }
}

struct SimpleTraversal;

impl Traveller for SimpleTraversal {
    fn traverse(&mut self, workloads: &[Box<ArrayList>; GROUP_SIZE]) -> u64 {
        let mut sum: u64 = 0;
        for workload in workloads.iter() {
            let mut pre_idx = 0;
            for _i in 0..ARRAY_SIZE {
                unsafe {
                    _mm_prefetch(
                        &workload.list[pre_idx] as *const Cell as *const i8,
                        _MM_HINT_T0,
                    );
                }
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

struct AsyncTraversal {
    executor: executor::Executor,
}

impl Traveller for AsyncTraversal {
    fn setup(&mut self) {}

    fn traverse(&mut self, workloads: &'static [Box<ArrayList>; GROUP_SIZE]) -> u64 {
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

impl AsyncTraversal {
    fn new() -> Self {
        AsyncTraversal {
            executor: executor::Executor::new(),
        }
    }
    async fn traverse_one(workload: &'static Box<ArrayList>) -> u64 {
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

fn benchmark(mut traveller: impl Traveller, options: &CommandLineOptions) {
    traveller.setup();

    for i in 0..options.repetition {
        let time_begin = Instant::now();
        let sum = traveller.traverse(&WORKLOADS);
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
    if options.traveller == "simple" {
        let simple_traversal = SimpleTraversal {};
        benchmark(simple_traversal, &options);
    } else if options.traveller == "async" {
        let async_traversal = AsyncTraversal::new();
        benchmark(async_traversal, &options);
    }
}
