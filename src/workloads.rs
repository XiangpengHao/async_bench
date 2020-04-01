use rand::seq::SliceRandom;
use rand::thread_rng;
use std::alloc::{alloc, Layout};
use std::mem;

pub const ARRAY_SIZE: usize = 1024 * 1024;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Cell {
    next_index: u64,
    _padding: [u64; 7],
}

impl Cell {
    pub fn set(&mut self, value: u64) {
        self.next_index = value;
    }
    pub fn get(&self) -> u64 {
        self.next_index
    }
}

#[repr(C)]
pub struct ArrayList {
    pub list: [Cell; ARRAY_SIZE],
}

impl ArrayList {
    pub fn new() -> Box<Self> {
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

    pub const fn ground_truth_sum(&self) -> u64 {
        ((0 + ARRAY_SIZE - 1) * ARRAY_SIZE / 2) as u64
    }

    fn _print_values(&self) {
        for elem in self.list.iter() {
            print!("{}\t", elem.next_index);
        }
        println!("");
    }
}
