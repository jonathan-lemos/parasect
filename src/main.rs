#![feature(btree_cursors)] // needed for numeric_range_set

mod collections;
mod parasect;
mod range;
mod task;
#[cfg(test)]
mod test_util;
mod threading;
mod ui;
mod util;

fn main() {
    println!("Hello, world!");
}
