# The page's build machine: x86-64 Ubuntu 24.04 with the pinned toolchain, wasm-bindgen and clang. Cargo
# hashes the build machine's type into every crate with a build script or a proc macro, and the hash
# reaches the binaries, so only a build on this machine type makes the bytes Willow serves. The OpenVM
# module also names the clang that compiled its C code, so the image pins Ubuntu's clang 18.1.3.
# reproduce.sh builds in this image on any other machine.
FROM ubuntu:24.04@sha256:2198e4c65076e9cb0b3d695219df667c0e3b0072da64ca3c2648aca17dcecaf5
# clang and llvm-ar compile zstd's C code (in OpenVM's verifier) for wasm32; gcc builds the host tools.
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates clang-18=1:18.1.3-1ubuntu1 curl gcc libc6-dev \
    llvm-18=1:18.1.3-1ubuntu1 perl python3 rsync \
  && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/rustup CARGO_HOME=/cargo PATH=/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  WASM_CC=clang-18 WASM_AR=llvm-ar-18
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --no-modify-path --profile minimal --default-toolchain nightly-2026-04-12 \
    --component rustfmt,clippy --target wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli --version 0.2.129 --locked && rm -rf /cargo/registry /cargo/git
