# valida-rs

This crate is to be used as a dependency in a Valida project. It provides the entry point for the program and an IO library that works on Valida.

## The `entrypoint!` macro

The `entrypoint!` macro:

- Sets up a deterministic random number generator: It ensures that when `rand` functions are called, they are fixed to a specified seed and thus are deterministic. This is required for the program to be provable.
- Creates a new entry point that wraps the user's main function: This is required because we need to make Rust call this `main` function, the standard Rust `main` function does not work on Valida.

### For projects that require `rand`

Use the `main` branch if you need randomness. Add the following to your `src/main.rs`:

```rust
#![no_main]

valida_rs::entrypoint!(main);
```

The `#![no_main]` (with `!`) is an inner attribute that applies to the entire crate, telling the Rust compiler not to look for a standard main function entry point. We need this because we are providing a custom entry point.

Add the following to your `Cargo.toml`:

```toml
[dependencies]
valida-rs = { git = "https://github.com/lita-xyz/valida-rs.git", branch = "main" }
getrandom = "0.2.15" # or the current version
rand = "0.8.5" # or the current version
```

## The `IO` library

This library provides common IO functions that work on Valida. See [io.rs](src/io.rs) for the full list of available functions. Note that not all standard library IO functions are supported yet. Also, most of the Rust standard `std::io` module is not supported at the moment. If you use them, they may silently not work.

To use these functions, simply prefix them with `valida_rs::io::`. For example, `valida_rs::io::println` instead of `println`. You can see more examples in the [rust-examples](https://github.com/lita-xyz/rust-examples) repository.

### For projects with no other dependencies

If you would like to use IO functionalities in your project, you will want to use the `no-deps` branch. If you need randomness and/or serialization/deserialization of data, use the `main` branch. See above for more details.

To use this branch, add the following to your `Cargo.toml`:

```toml
[dependencies]
valida-rs = { git = "https://github.com/lita-xyz/valida-rs.git", branch = "no-deps" }
```

Also, in your `src/main.rs`, add the following:

```rust
#![no_main]

#[no_mangle]  
pub fn main() {
    // ...
}
```

The `#[no_mangle]` attribute tells the compiler not to mangle (rename) the function name during compilation. We need this because the Valida runtime looks for a function specifically named "main".
