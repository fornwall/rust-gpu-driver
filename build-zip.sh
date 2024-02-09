#!/bin/bash
set -e -u

RUST_GPU_REVISION=$(cat src/rust-gpu-revision.txt)
: ${TARGET="x86_64-unknown-linux-gnu"}

CHANNEL=$(cat src/nightly-channel.txt)
STRIP=echo

if [[ $TARGET = *apple* ]]; then
    LIBRUSTC_CODEGEN_SPIRV=librustc_codegen_spirv.dylib
else
    LIBRUSTC_CODEGEN_SPIRV=librustc_codegen_spirv.so
fi

# Install nightly toolchain
rustup uninstall $CHANNEL # To avoid old targets
rustup install --profile minimal $CHANNEL
# rustup install $CHANNEL
# rustup component add rust-src rustc-dev llvm-tools --toolchain $CHANNEL
rustup component add rust-src --toolchain $CHANNEL
RUSTUP_TOOLCHAIN_FOLDER=${CHANNEL}-${TARGET}
RUSTUP_TOOLCHAIN_PATH=$HOME/.rustup/toolchains/$RUSTUP_TOOLCHAIN_FOLDER
ls -lha $HOME/.rustup/toolchains
du -sh $RUSTUP_TOOLCHAIN_PATH

# Bundle toolchain up before building (which bloats toolchain dir)
BUILD_DIR=target/rust-gpu-compiler-distribution
rm -Rf $BUILD_DIR
mkdir -p $BUILD_DIR/share
cd $BUILD_DIR/share
cp -Rf $RUSTUP_TOOLCHAIN_PATH rust-gpu-toolchain
cp ../../../shaders/example.rs rust-gpu-toolchain
# Trim away not needed parts:
# rm -Rf rust-gpu-toolchain/lib/rustlib
rm -Rf rust-gpu-toolchain/{etc,libexec,share}
rm rust-gpu-toolchain/bin/{rust-gdb,rust-gdbgui,rust-lldb,rustdoc}
$STRIP rust-gpu-toolchain/bin/*
cd ../../../

# Build rust-gpu-compiler
cargo build --release --target $TARGET

# Build rust
RUSTGPU_DIR=rust-gpu-checkout
cd target
if [ ! -d $RUSTGPU_DIR ]; then
    git clone https://github.com/EmbarkStudios/rust-gpu $RUSTGPU_DIR
fi
cd $RUSTGPU_DIR
git reset --hard $RUST_GPU_REVISION
cargo build --release -p rustc_codegen_spirv --target $TARGET
cd ../..

# Bundle the rust-gpu-compiler binary and librustc_codegen_spirv.so and create zip
cd $BUILD_DIR
mkdir bin lib
cp ../$TARGET/release/rust-gpu-compiler bin/rust-gpu
cp ../$RUSTGPU_DIR/target/$TARGET/release/$LIBRUSTC_CODEGEN_SPIRV lib/
$STRIP lib/$LIBRUSTC_CODEGEN_SPIRV bin/rust-gpu
zip ../../rust-gpu-compiler-$TARGET.zip -r .
cd ../../
