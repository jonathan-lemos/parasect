#![feature(btree_cursors)] // needed for numeric_range_set

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
