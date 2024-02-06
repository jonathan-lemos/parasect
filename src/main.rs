#![feature(btree_cursors)] // needed for numeric_range_set

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;
extern crate core;

mod algorithms;
mod task;
mod collections;
#[cfg(test)]
mod test_util;
mod range;
mod util;

fn main() {
    println!("Hello, world!");
}
