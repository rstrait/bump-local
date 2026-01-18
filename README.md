# `bump-local`

A `Sync + Send` allocator wrapper around [bumpalo](https://docs.rs/bumpalo) using per-thread bump allocators.

## Why?

`bumpalo::Bump` is fast but not thread-safe. This crate provides a thread-safe interface by giving each thread its own `Bump` instance via thread-local storage,
combining bumpalo's allocation speed with `Sync + Send` semantics.

This crate is useful when you need to pass a custom allocator to a library that requires `Sync + Send`.
If you're writing multithreaded code from scratch, consider creating per-thread allocators upfront and using them instead.

## Usage

```rust
use bump_local::Bump;

let mut bump = Bump::new();
let bump_clone = bump.clone();
let handle = std::thread::spawn(move || {
    // This thread gets its own bump allocator instance
    let local = bump_clone.local();
    local.as_inner().alloc(42);
    // bump_clone is dropped when thread finishes
});

// Wait for thread to finish
handle.join().unwrap();

// Now safe to reset - all clones are dropped
bump.reset_all().unwrap();
```

Check out the [examples](examples/) directory for examples using rayon and bumpalo collections.

## Minimum Supported Rust Version (MSRV)

This crate requires Rust 1.71.1 or later.

## License

MIT OR Apache-2.0
