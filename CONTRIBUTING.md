# Contributing
If you want to contribute to this project, thanks! This document goes over some guidelines you should
abide by when contributing.

## Prerequisites
Make sure you have the stable Rust toolchain and that your Rust version is at least v1.61.0.

## Basic Guidelines
- Test to see if all tests still work using `cargo test --features=all`.
- Format your code using `cargo fmt`.
- Run clippy: `cargo clippy --workspace --all-features -- -D clippy::all -D clippy::pedantic -D clippy::nursery -D clippy::cargo
`

There are workflows that check for all three of these to make sure they work.

## Your Cargo.toml
Make sure you use the **GitHub version** instead of the one on crates.io since it has the most 
up-to-date code:

```toml
[dependencies.ril]
git = "https://github.com/jay3332/ril"
branch = "main"
features = ["all"] # Important!
```

Also, make sure **all** features are enabled.

### Testing and benchmarking
Make sure to also enable all features when running tests, clippy\*, and benchmarking:

- `cargo test --features=all`
- `cargo bench --features=all`
- `cargo clippy --workspace --all-features -- -D clippy::all -D clippy::pedantic -D clippy::nursery -D clippy::cargo`

\* When running clippy, make sure you run with all the above lints enabled.