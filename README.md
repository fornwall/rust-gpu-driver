[![CI](https://github.com/fornwall/rust-gpu-driver/workflows/CI/badge.svg)](https://github.com/fornwall/rust-gpu-driver/actions?query=workflow%3ACI)

# rust-gpu-driver
Experiment to make [rust-gpu](https://github.com/EmbarkStudios/rust-gpu) more accessible as a GPU shading language in various projects.

**DISCLAIMER**: This is an unstable experiment - use at your own risk and expect things not working and breaking over time.

## Installation from homebrew

Currently Linux x86-64 and macOS aarch64 binaries can be installed using [Homebrew](https://brew.sh/) on macOS and Linux:

```sh
$ brew install fornwall/tap/rust-gpu
```

Windows users can use the Linux package through `WSL` for now.

## Usage

With a rust-gpu shader file `shader.rs` such as:

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

The `-o`/`--output` option can be used to specify the path:

```sh
$ rust-gpu -o build/output.spv shader.rs
$ rust-gpu -o - shader.rs | spirv-dis
```

Dependencies can be specified in the script using the [cargo-script](https://rust-lang.github.io/rfcs/3424-cargo-script.html) syntax with an embedded part of the manifest:

```rust
//! ```cargo
//! [dependencies]
//! either = { version = "1", default-features = false }
//! ```
#![no_std]

use spirv_std::glam::{vec4, Vec4};
use spirv_std::spirv;
use either::Either;

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    let e: Either<f32, f32> = Either::Left(12.0);
    *output = vec4(e.left().unwrap_or(0.0), 2.0, 0.0, 1.0);
}
```

## SPIR-V targets
A [SPIR-V target](https://embarkstudios.github.io/rust-gpu/book/platform-support.html) can be specified using `-t`/`--target` (default value: `spirv-unknown-vulkan1.1`):

```sh
$ rust-gpu -t spirv-unknown-vulkan1.2 shader.rs
```
