# valida-rs

This crate provides a backend for `getrandom` crate version family `0.2.x`.

This crate won't work with `getrandom` version `0.3.x`.

## The `entrypoint!` macro

The `entrypoint!` macro:

- Sets up a deterministic random number generator. It ensures that when `rand` functions are called, they are fixed to a specified seed and thus are deterministic. This is required for the program to be provable.
- Creates a new entry point that wraps the user's main function.

### For projects that require `getrandom` / `rand`

Add the following to your `src/main.rs`:

```rust
#![no_main]

valida_rs::entrypoint!(main);
```

The `#![no_main]` (with `!`) is an inner attribute that applies to the entire crate, telling the Rust compiler not to look for a standard main function entry point. We need this because we are providing a custom entry point.

Add the following to your `Cargo.toml`:

```toml
[dependencies]
valida-rs = { git = "https://github.com/lita-xyz/valida-rs.git" }
getrandom = "0.2.15" # or any 0.2.x
rand = "0.8.5"
```

## Issue reporting

Any issues report at https://github.com/lita-xyz/valida-releases/issues
