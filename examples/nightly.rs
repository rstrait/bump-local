//! This example demonstrates using bump-local with rayon for parallel processing.
//!
//! Run with:
//!   cargo +nightly run --example nightly --features allocator_api

#![feature(allocator_api)]

use bump_local::Bump;
use rayon::prelude::*;

fn main() {
    let bump = Bump::builder().bump_capacity(1024 * 1024).build();

    println!("Processing data in parallel with bump allocator...\n");

    // Process numbers in parallel, each thread allocates its own Vec
    let results: Vec<_> = (0..10)
        .into_par_iter()
        .map(|i| {
            // Each thread gets its own thread-local bump allocator
            let local = bump.local();
            let mut vec = Vec::new_in(bump.clone());

            // Allocate some data
            for j in 0..1000 {
                vec.push(i * 1000 + j);
            }

            // Do some work
            let sum: i32 = vec.iter().sum();
            let cap = local.as_inner().chunk_capacity();
            (std::thread::current().id(), i, vec.len(), cap, sum)
        })
        .collect();

    for (thread, id, len, cap, sum) in results {
        println!("Thread {thread:?} ({id}): allocated {len} items, chunk capacity = {cap}, sum = {sum}");
    }
}
