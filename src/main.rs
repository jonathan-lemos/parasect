#![feature(btree_cursors)] // needed for numeric_range_set

extern crate core;

use std::process::ExitCode;

mod collections;
mod parasect;
mod range;
mod setup;
mod task;
#[cfg(test)]
mod test_util;
mod threading;
mod ui;
mod util;

fn main() -> ExitCode {
    println!("Hello, world!");
    ExitCode::SUCCESS
}
