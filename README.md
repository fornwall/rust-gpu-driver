[![CI](https://github.com/fornwall/rust-gpu-compiler/workflows/CI/badge.svg)](https://github.com/fornwall/rust-gpu-compiler/actions?query=workflow%3ACI)

# rust-gpu-compiler
Experiment to make [rust-gpu](https://github.com/EmbarkStudios/rust-gpu) more accessible as a general shading language alternative.

**DISCLAIMER**: This is an unstable experiment - use at your own risk and expect things not working and breaking over time.

## Installation from homebrew

Currently 64-bit arm and x86 versions can be installed using [Homebrew](https://brew.sh/) on macOS and Linux:

```sh
$ brew install fornwall/tap/rust-gpu
[...]
```

## Getting a binary directly

TODO

## Usage

Create a `shader.rs` file containing:

```rust
#![no_std]

use spirv_std::spirv;
use spirv_std::glam::{vec4, Vec4};

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    *output = vec4(1.0, 0.0, 0.0, 1.0);
}
```

Build a `shader.spv` file from it using:

```sh
$ rust-gpu shader.rs
```

Dependencies can be specified in the script using the [cargo-script](https://rust-lang.github.io/rfcs/3424-cargo-script.html) syntax with an embedded part of the manifest:

```rust
#![no_std]

//! ```cargo
//! [dependencies]
//! clap = { version = "4.2", features = ["derive"] }
//! ```

use spirv_std::spirv;
use spirv_std::glam::{vec4, Vec4};

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    *output = vec4(1.0, 0.0, 0.0, 1.0);
}
```
