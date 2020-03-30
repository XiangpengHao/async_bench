use crate::executor::{MemoryAccessFuture, Task};
use arr_macro::arr;
use core::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::alloc::{alloc, Layout};
use std::mem;
use std::time::Instant;
#[macro_use]
extern crate lazy_static;

pub mod executor;

const ARRAY_SIZE: usize = 1024 * 1024;
const REPETITION: usize = 4;

trait Traveller {
    fn setup(&mut self);
    fn traverse(&mut self, workloads: &'static [Box<ArrayList>; REPETITION]) -> u64;
    fn get_name(&self) -> &'static str;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Cell {
    next_index: u64,
    _padding: [u64; 7],
}

impl Cell {
    fn new() -> Self {
        Cell {
            next_index: 0,
            _padding: [0; 7],
        }
    }
    fn set(&mut self, value: u64) {
        self.next_index = value;
    }
    fn get(&self) -> u64 {
        self.next_index
    }
}

lazy_static! {
    static ref WORKLOADS: [Box<ArrayList>; REPETITION] = arr![ArrayList::new(); 4];
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
    fn traverse(&mut self, workloads: &[Box<ArrayList>; REPETITION]) -> u64 {
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

struct AsyncTraversal {
    executor: executor::Executor,
}

impl Traveller for AsyncTraversal {
    fn setup(&mut self) {}

    fn traverse(&mut self, workloads: &'static [Box<ArrayList>; REPETITION]) -> u64 {
        for i in 0..REPETITION {
            self.executor
                .spawn(Task::new(AsyncTraversal::traverse_one(&workloads[i])));
        }
        self.executor.run_ready_task()
    }

    fn get_name(&self) -> &'static str {
        "AsyncTraversal"
    }
}

impl AsyncTraversal {
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
            MemoryAccessFuture {}.await;
            let value = workload.list[pre_idx].get();
            pre_idx = value as usize;
            sum += value;
        }
        sum
    }
}

fn benchmark(traveller: &mut impl Traveller) {
    traveller.setup();

    let time_begin = Instant::now();
    let sum = traveller.traverse(&WORKLOADS);
    let elapsed = time_begin.elapsed().as_nanos();

    println!("{}: {} ns", traveller.get_name(), elapsed);
    assert_eq!(sum, WORKLOADS[0].ground_truth_sum() * 4);
}

fn main() {
    let mut simple_traversal = SimpleTraversal {};
    benchmark(&mut simple_traversal);

    let mut async_traversal: AsyncTraversal = AsyncTraversal {
        executor: executor::Executor::new(),
    };
    benchmark(&mut async_traversal);
}
