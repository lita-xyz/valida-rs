# valida-rs
The entry point for Rust projects to be run on Valida

Use this branch if you don't need any dependencies that we have specifically patched for Valida to work. If you need randomness and/or serialization/deserialization of data, use the `main` branch.

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
