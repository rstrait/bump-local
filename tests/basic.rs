use std::{
    alloc::Layout,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

use bump_local::Bump;

macro_rules! wg_new {
    ($count:expr) => {
        Arc::new((Mutex::new($count), Condvar::new()))
    };
}

macro_rules! wg_done {
    ($wg:ident) => {
        let (lock, cvar) = &*$wg;
        let mut count = lock.lock().unwrap();
        *count -= 1;
        if *count == 0 {
            cvar.notify_all();
        }
        drop(count);
    };
}

macro_rules! wg_wait {
    ($wg:ident) => {
        let (lock, cvar) = &*$wg;
        let mut count = lock.lock().unwrap();
        while *count > 0 {
            count = cvar.wait(count).unwrap();
        }
        drop(count);
    };
}

macro_rules! broadcast_new {
    ($type:ty) => {
        Arc::new((Mutex::new(None::<$type>), Condvar::new()))
    };
}

macro_rules! broadcast_wait {
    ($signal:ident) => {{
        let (lock, cvar) = &*$signal;
        let mut value = lock.lock().unwrap();
        while (*value).is_none() {
            value = cvar.wait(value).unwrap();
        }
        let result = (*value).as_ref().unwrap().clone();
        drop(value);
        result
    }};
}

macro_rules! broadcast_send {
    ($signal:ident, $value:expr) => {
        let (lock, cvar) = &*$signal;
        let mut guard = lock.lock().unwrap();
        *guard = Some($value);
        cvar.notify_all();
        drop(guard);
    };
}

#[test]
fn reset_all() {
    let mut bump = Bump::builder().bump_capacity(100).build();

    let layouts = [
        Layout::new::<i8>(),
        Layout::new::<i16>(),
        Layout::new::<i32>(),
    ];

    let mut threads = Vec::<JoinHandle<()>>::with_capacity(layouts.len());
    let next_bumps = broadcast_new!(Bump);
    let ready_for_next = wg_new!(layouts.len());
    for layout in layouts {
        let next_bumps = next_bumps.clone();
        let ready_for_next = ready_for_next.clone();
        let bump = bump.clone();

        threads.push(thread::spawn(move || {
            let initial_capacity = bump.local().as_inner().chunk_capacity();

            let _ = bump.local().as_inner().alloc_layout(layout);
            let capacity_after_alloc = bump.local().as_inner().chunk_capacity();

            assert!(
                capacity_after_alloc < initial_capacity,
                "Thread 1: first={}, second={}",
                initial_capacity,
                capacity_after_alloc
            );

            drop(bump);
            wg_done!(ready_for_next);

            let bump = broadcast_wait!(next_bumps);
            let capacity_after_reset = bump.local().as_inner().chunk_capacity();
            assert_eq!(capacity_after_reset, initial_capacity);
        }));
    }

    wg_wait!(ready_for_next);
    bump.reset_all().unwrap();

    broadcast_send!(next_bumps, bump);

    for handle in threads {
        handle.join().unwrap()
    }
}

#[test]
fn local_reuse() {
    let bump = Bump::builder().bump_capacity(100).build();

    let layouts = [
        Layout::new::<i8>(),
        Layout::new::<i16>(),
        Layout::new::<i32>(),
    ];

    let mut threads = Vec::<JoinHandle<()>>::with_capacity(layouts.len());
    let next_bumps = broadcast_new!(Bump);
    let ready_for_next = wg_new!(layouts.len());
    for layout in layouts {
        let next_bumps = next_bumps.clone();
        let ready_for_next = ready_for_next.clone();
        let bump = bump.clone();

        threads.push(thread::spawn(move || {
            let initial_capacity = bump.local().as_inner().chunk_capacity();
            let _ = bump.local().as_inner().alloc_layout(layout);
            drop(bump);
            wg_done!(ready_for_next);

            let bump = broadcast_wait!(next_bumps);
            let capacity_after_alloc = bump.local().as_inner().chunk_capacity();
            assert_eq!(capacity_after_alloc, initial_capacity - layout.size());
        }));
    }

    wg_wait!(ready_for_next);
    broadcast_send!(next_bumps, bump);

    for handle in threads {
        handle.join().unwrap()
    }
}
