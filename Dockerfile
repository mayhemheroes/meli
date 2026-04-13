# Build Stage
FROM rustlang/rust:nightly AS builder

## Install build dependencies.
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y cmake clang curl pkg-config libssl-dev libsqlite3-dev

## Add source code to the build stage.
ADD . /meli
WORKDIR /meli

RUN cargo install cargo-fuzz
RUN cd fuzz && cargo fuzz build

# Package Stage
FROM ubuntu:24.04

COPY --from=builder meli/fuzz/target/x86_64-unknown-linux-gnu/release/envelope_parse /

RUN apt-get update -y && apt-get install -y libssl-dev libsqlite3-dev
