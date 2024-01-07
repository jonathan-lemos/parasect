#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod algorithms;
mod task;

fn main() {
    println!("Hello, world!");
}
