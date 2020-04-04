use rand::seq::SliceRandom;
use rand::thread_rng;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Cell {
    next_index: u64,
    _padding: [u64; 7],
}

impl Cell {
    pub fn new(next_idx: u64) -> Self {
        Cell {
            next_index: next_idx,
            _padding: [0; 7],
        }
    }
    pub fn set(&mut self, value: u64) {
        self.next_index = value;
    }
    pub fn get(&self) -> u64 {
        self.next_index
    }
}

pub struct ArrayList {
    pub list: Vec<Cell>,
}

impl ArrayList {
    pub fn new(array_size: usize) -> Self {
        let mut workload_list = ArrayList {
            list: vec![Cell::new(0); array_size],
        };
        let mut temp_values: Vec<u64> = Vec::with_capacity(array_size - 1);
        for i in 1..array_size {
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

    pub fn ground_truth_sum(&self) -> u64 {
        ((0 + self.list.len() - 1) * self.list.len() / 2) as u64
    }

    fn _print_values(&self) {
        for elem in self.list.iter() {
            print!("{}\t", elem.next_index);
        }
        println!("");
    }
}
