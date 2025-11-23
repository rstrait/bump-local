use std::{
    cell::UnsafeCell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use thread_local::ThreadLocal;

mod error;
pub use error::ResetError;

struct ThreadGuard {
    alive: Arc<AtomicBool>,
}

impl ThreadGuard {
    fn new() -> Self {
        Self {
            alive: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}

thread_local! {
    static THREAD_GUARD: ThreadGuard = ThreadGuard::new();
}

/// A thread-safe bump allocator that provides `Sync + Send` semantics.
///
/// Each thread gets its own `BumpLocal` instance.
#[derive(Default, Clone)]
pub struct Bump {
    inner: Arc<BumpInner>,
}

impl Bump {
    /// Creates a new empty `Bump` allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a builder for configuring a `Bump` allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_local::Bump;
    ///
    /// let bump = Bump::builder()
    ///     .threads_capacity(8)
    ///     .bump_capacity(4096)
    ///     .build();
    /// ```
    pub fn builder() -> BumpBuilder {
        BumpBuilder::new()
    }

    /// Returns the allocator for the current thread,
    /// or creates it if it doesn't exist.
    ///
    /// If the `reset_all` was called earlier,
    /// this would reset the current thread's allocator which is O(1).
    #[inline]
    pub fn local(&self) -> &BumpLocal {
        self.inner.local()
    }

    /// Resets all threads' bump allocators, deallocating all previously allocated memory.
    ///
    /// # Safety Contract
    ///
    /// - At the moment of reset it must be the only handle to the Bump.
    /// - Like `bumpalo::Bump::reset()`, callers must ensure no references to allocated memory
    ///   are used after calling this method.
    /// - This does not run any `Drop` implementations.
    #[inline]
    pub fn reset_all(&mut self) -> Result<(), ResetError> {
        match Arc::get_mut(&mut self.inner) {
            Some(inner) => {
                inner.reset_all();
                Ok(())
            }
            None => Err(ResetError),
        }
    }
}

/// Builder for configuring a `Bump` allocator.
#[derive(Default)]
pub struct BumpBuilder {
    threads_capacity: Option<usize>,
    bump_alloc_limit: Option<usize>,
    bump_capacity: usize,
}

impl BumpBuilder {
    /// Creates a new `BumpBuilder` with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the initial capacity hint for the number of threads that will access this allocator.
    ///
    /// This can reduce allocations in the underlying `ThreadLocal` storage when you know
    /// how many threads will use the allocator.
    pub fn threads_capacity(mut self, capacity: usize) -> Self {
        self.threads_capacity = Some(capacity);
        self
    }

    /// Sets the allocation limit for each per-thread bump allocator.
    ///
    /// Once the limit is reached, further allocations will fail.
    pub fn bump_allocation_limit(mut self, limit: usize) -> Self {
        self.bump_alloc_limit = Some(limit);
        self
    }

    /// Sets the initial capacity for each per-thread bump allocator.
    ///
    /// This pre-allocates memory for each thread's allocator, which can improve performance
    /// if you know approximately how much memory each thread will need.
    pub fn bump_capacity(mut self, capacity: usize) -> Self {
        self.bump_capacity = capacity;
        self
    }

    /// Builds the `Bump` allocator with the configured parameters.
    pub fn build(self) -> Bump {
        Bump {
            inner: Arc::new(BumpInner {
                locals: match self.threads_capacity {
                    Some(cap) => ThreadLocal::with_capacity(cap),
                    None => ThreadLocal::new(),
                },
                capacity: self.bump_capacity,
                alloc_limit: self.bump_alloc_limit,
            }),
        }
    }
}

/// Per-thread wrapper around a `bumpalo::Bump` allocator.
pub struct BumpLocal {
    inner: UnsafeCell<Option<BumpLocalInner>>,
}

impl BumpLocal {
    fn new(capacity: usize, limit: Option<usize>, thread_alive: Arc<AtomicBool>) -> Self {
        let bump = bumpalo::Bump::with_capacity(capacity);
        bump.set_allocation_limit(limit);

        Self {
            inner: UnsafeCell::new(Some(BumpLocalInner {
                inner: bump,
                thread_alive,
            })),
        }
    }

    #[inline]
    pub fn needs_init(&self) -> bool {
        // SAFETY: ThreadLocal ensures single-thread access to this BumpLocal.
        unsafe { (*self.inner.get()).is_none() }
    }

    #[cold]
    pub fn init(&self, capacity: usize, limit: Option<usize>, thread_alive: Arc<AtomicBool>) {
        let bump = bumpalo::Bump::with_capacity(capacity);
        bump.set_allocation_limit(limit);

        // SAFETY: ThreadLocal ensures single-thread access to this BumpLocal.
        unsafe {
            *self.inner.get() = Some(BumpLocalInner {
                inner: bump,
                thread_alive,
            })
        }
    }

    /// Returns a reference to the underlying `bumpalo::Bump` allocator.
    ///
    /// The returned reference provides access to all `bumpalo::Bump` allocation methods.
    #[inline]
    pub fn as_inner(&self) -> &bumpalo::Bump {
        // SAFETY:
        // - BumpLocal is only constructed inside ThreadLocal,
        //   which ensures it's only accessed by one thread.
        // - The returned reference is !Send since bumpalo::Bump is !Sync.
        // - The reference lifetime is bound to the parent Bump allocator.
        unsafe { &(*self.inner.get()).as_ref().unwrap().inner }
    }

    /// Resets the allocator.
    #[inline]
    pub fn reset(&self) {
        // SAFETY: ThreadLocal ensures single-thread access to this BumpLocal.
        unsafe {
            (*self.inner.get()).as_mut().unwrap().inner.reset();
        }
    }

    #[cold]
    fn clear(&mut self) {
        #[cold]
        fn drop_inner(bump: &mut BumpLocal) {
            // SAFETY: ThreadLocal ensures single-thread access to this BumpLocal.
            unsafe {
                let _ = (*bump.inner.get()).take();
            }
        }

        // SAFETY: ThreadLocal ensures single-thread access to this BumpLocal.
        let inner = unsafe { &*self.inner.get() };
        let Some(inner) = inner.as_ref() else {
            return;
        };

        if inner.thread_alive.load(Ordering::Acquire) {
            self.reset();
        } else {
            drop_inner(self);
        }
    }
}

struct BumpLocalInner {
    inner: bumpalo::Bump,
    thread_alive: Arc<AtomicBool>,
}

// Shared `Bump` state.
#[derive(Default)]
struct BumpInner {
    locals: ThreadLocal<BumpLocal>,
    capacity: usize,
    alloc_limit: Option<usize>,
}

impl BumpInner {
    #[inline]
    fn local(&self) -> &BumpLocal {
        let bump = self.locals.get_or(|| {
            let thread_alive = THREAD_GUARD.with(|guard| guard.alive.clone());
            BumpLocal::new(self.capacity, self.alloc_limit, thread_alive)
        });

        if bump.needs_init() {
            self.reinit_local(bump);
        }

        bump
    }

    #[cold]
    fn reinit_local(&self, bump: &BumpLocal) {
        let thread_alive = THREAD_GUARD.with(|guard| guard.alive.clone());
        bump.init(self.capacity, self.alloc_limit, thread_alive);
    }

    #[inline]
    fn reset_all(&mut self) {
        for local in self.locals.iter_mut() {
            local.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn thread_guard_sets_alive_false_on_drop() {
        let handle = thread::spawn(move || THREAD_GUARD.with(|g| g.alive.clone()));

        let alive = handle.join().unwrap();
        assert!(!alive.load(Ordering::Acquire));
    }

    #[test]
    fn reset_resets_alive_thread() {
        let mut bump = Bump::builder().bump_capacity(100).build();

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = {
            let bump = bump.clone();
            thread::spawn(move || {
                let _ = bump.local().as_inner().alloc(1_u8);
                let capacity_before = bump.local().as_inner().chunk_capacity();
                drop(bump);

                tx.send(capacity_before).unwrap();
                thread::park();
            })
        };

        let capacity_before = rx.recv().unwrap();

        // reset while thread is still alive
        bump.reset_all().unwrap();

        // check if bump was reset, not dropped
        let inner = Arc::get_mut(&mut bump.inner).unwrap();
        let locals: Vec<_> = inner.locals.iter_mut().collect();
        assert_eq!(locals.len(), 1);
        let local = locals.first().unwrap();
        assert!(!local.needs_init());
        assert!(local.as_inner().chunk_capacity() > capacity_before);

        handle.thread().unpark();
        handle.join().unwrap();
    }

    #[test]
    fn reset_drops_dead_thread_bump() {
        let mut bump = Bump::builder().bump_capacity(100).build();

        let handle = {
            let bump = bump.clone();
            thread::spawn(move || {
                let _ = bump.local().as_inner().alloc(1_u8);
            })
        };

        handle.join().unwrap();

        // reset_all should detect dead thread and drop its bump
        bump.reset_all().unwrap();

        let inner = Arc::get_mut(&mut bump.inner).unwrap();
        let locals: Vec<_> = inner.locals.iter_mut().collect();
        assert_eq!(locals.len(), 1);
        let local = locals.first().unwrap();
        assert!(local.needs_init());
    }
}
