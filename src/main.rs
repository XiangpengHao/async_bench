#![feature(const_generics)]
#![feature(const_in_array_repeat_expressions)]

use std::rc::Rc;
use std::time::Instant;
use structopt::StructOpt;
use travellers::{AsyncTraversal, SimpleTraversal, Traveller};
use workloads::{ArrayList, Cell};

pub mod executor;
pub mod travellers;
pub mod workloads;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

const GROUP_SIZE: usize = 4;

fn benchmark(
    workloads: Vec<Rc<ArrayList>>,
    traveller: impl Traveller,
    options: &CommandLineOptions,
) {
    for i in 0..options.repetition {
        let time_begin = Instant::now();
        let sum = traveller.traverse(&workloads);
        let elapsed = time_begin.elapsed().as_nanos();

        println!("{}#{}: {} ns", traveller.get_name(), i, elapsed);
        assert_eq!(sum, workloads[0].ground_truth_sum() * GROUP_SIZE as u64);
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "async_bench")]
struct CommandLineOptions {
    #[structopt(short, long)]
    traveller: String,

    #[structopt(short, long, default_value = "3")]
    repetition: i32,

    #[structopt(short, long, default_value = "1048576")]
    array_size: i32,
}

fn main() {
    let options = CommandLineOptions::from_args();

    let workloads: Vec<Rc<ArrayList>> = [0; GROUP_SIZE]
        .iter()
        .map(|_| {
            let list = ArrayList::new(options.array_size as usize);
            Rc::new(list)
        })
        .collect::<Vec<Rc<ArrayList>>>();

    if options.traveller == "sync" {
        let traveller = SimpleTraversal {};
        benchmark(workloads, traveller, &options);
    } else if options.traveller == "async" {
        let traveller = AsyncTraversal {};
        benchmark(workloads, traveller, &options);
    }
}
