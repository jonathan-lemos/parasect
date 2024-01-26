#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod algorithms;
mod task;
mod collections;
#[cfg(test)]
mod test_util;
mod range;

fn main() {
    println!("Hello, world!");
}
