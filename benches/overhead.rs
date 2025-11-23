#![feature(test)]

extern crate test;

use std::sync::{Arc, Mutex};

use bump_local::Bump;
use test::{black_box, Bencher};

#[allow(dead_code)]
#[derive(Default)]
struct Small(u8);

#[allow(dead_code)]
#[derive(Default)]
struct Big([usize; 32]);

const ALLOCATIONS: usize = 10_000;

fn alloc_bumpalo<T: Default>() {
    let bump = bumpalo::Bump::with_capacity(ALLOCATIONS * std::mem::size_of::<T>());
    for _ in 0..ALLOCATIONS {
        let arena = black_box(&bump);
        let val: &mut T = arena.alloc(black_box(Default::default()));
        black_box(val);
    }
}

fn alloc_bump_local<T: Default>() {
    let bump = Bump::builder()
        .bump_capacity(ALLOCATIONS * std::mem::size_of::<T>())
        .build();

    for _ in 0..ALLOCATIONS {
        let local = black_box(bump.local());
        let val: &mut T = local.as_inner().alloc(black_box(Default::default()));
        black_box(val);
    }
}

// Arc<Mutex<Bump>> approach.
//
// Even in single-threaded benchmarks, mutex lock/unlock adds ~4.7ns overhead per allocation
// (your system can show different results).
// In multi-threaded scenarios with contention, this overhead would be much worse.
fn alloc_mutex_bump<T: Default>() {
    let bump = Arc::new(Mutex::new(
        bumpalo::Bump::with_capacity(ALLOCATIONS * std::mem::size_of::<T>())
    ));

    for _ in 0..ALLOCATIONS {
        let guard = black_box(bump.lock().unwrap());
        let val: &mut T = guard.alloc(black_box(Default::default()));
        black_box(val);
    }
}

#[bench]
fn bumpalo_small(b: &mut Bencher) {
    b.iter(|| {
        alloc_bumpalo::<Small>();
    });
}

#[bench]
fn bumpalo_big(b: &mut Bencher) {
    b.iter(|| {
        alloc_bumpalo::<Big>();
    });
}

#[bench]
fn bump_local_small(b: &mut Bencher) {
    b.iter(|| {
        alloc_bump_local::<Small>();
    });
}

#[bench]
fn bump_local_big(b: &mut Bencher) {
    b.iter(|| {
        alloc_bump_local::<Big>();
    });
}

#[bench]
fn mutex_bump_small(b: &mut Bencher) {
    b.iter(|| {
        alloc_mutex_bump::<Small>();
    });
}

#[bench]
fn mutex_bump_big(b: &mut Bencher) {
    b.iter(|| {
        alloc_mutex_bump::<Big>();
    });
}
