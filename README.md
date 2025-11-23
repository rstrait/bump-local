# `bump-local`

A `Sync + Send` allocator wrapper around [bumpalo](https://docs.rs/bumpalo) using per-thread bump allocators.

## Why?

`bumpalo::Bump` is fast but not thread-safe. This crate provides a thread-safe interface by giving each thread its own `Bump` instance via thread-local storage,
combining bumpalo's allocation speed with `Sync + Send` semantics.

## Usage

```rust
use bump_local::Bump;

let allocator = Bump::new();
// Use from any thread - each thread gets its own Bump
```

## License

MIT OR Apache-2.0
