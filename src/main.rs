#![feature(btree_cursors)] // needed for numeric_range_set

extern crate core;

mod collections;
mod parasect;
mod range;
mod task;
#[cfg(test)]
mod test_util;
mod util;

fn main() {
    println!("Hello, world!");
}
