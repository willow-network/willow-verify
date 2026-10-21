# The page's build machine: x86-64 Linux with the pinned toolchain and wasm-bindgen. Cargo hashes
# the build machine's type into every crate with a build script or a proc macro, and the hash
# reaches the binaries, so only a build on this machine type makes the bytes Willow serves.
# reproduce.sh builds in this image on any other machine.
FROM debian:bookworm-slim@sha256:7ff6ba31527b9e336162fa061746c366c4b2032f4d2cff2078bea43f78efe3f8
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl gcc libc6-dev perl python3 rsync \
  && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/rustup CARGO_HOME=/cargo PATH=/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --no-modify-path --profile minimal --default-toolchain nightly-2026-04-12 \
    --component rustfmt,clippy --target wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli --version 0.2.103 --locked && rm -rf /cargo/registry /cargo/git
