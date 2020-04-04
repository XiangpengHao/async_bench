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

fn benchmark<'a>(
    workloads: &'a [ArrayList; GROUP_SIZE],
    mut traveller: impl Traveller<'a>,
    options: &CommandLineOptions,
) {
    traveller.setup();

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

    let workloads: [ArrayList; GROUP_SIZE] = unsafe {
        let mut data: [std::mem::MaybeUninit<ArrayList>; GROUP_SIZE] =
            std::mem::MaybeUninit::uninit().assume_init();
        for elem in &mut data[..] {
            std::ptr::write(
                elem.as_mut_ptr(),
                ArrayList::new(options.array_size as usize),
            );
        }
        std::mem::transmute::<_, [ArrayList; GROUP_SIZE]>(data)
    };

    if options.traveller == "sync" {
        let traveller = SimpleTraversal {};
        benchmark(&workloads, traveller, &options);
    } else if options.traveller == "async" {
        let traveller: AsyncTraversal = AsyncTraversal::new();
        benchmark(&workloads, traveller, &options);
    }
}
