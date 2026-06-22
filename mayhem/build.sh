#!/usr/bin/env bash
#
# meli/mayhem/build.sh — build meli's cargo-fuzz target(s) as sanitized libFuzzer binaries,
# replicating OSS-Fuzz's Rust path (base-builder-rust `compile` + a `cargo fuzz build -O`).
#
# meli is a Rust e-mail client; the fuzzed code is its `melib` library. The fuzz target drives
# `melib::Envelope::from_bytes` — the RFC5322 e-mail/MIME parser.
#
# We build our OWN additive cargo-fuzz crate (mayhem/fuzz/), NOT upstream's fuzz/. Upstream's
# fuzz/Cargo.toml pins libfuzzer-sys 0.3 and depends on melib with ALL default features (imap,
# nntp, smtp, ...), which pulls imap-codec — that fails to compile on the pinned fuzzing nightly.
# mayhem/fuzz builds the SAME entry point against melib's minimal parser core
# (default-features = false → no imap/smtp/tls/sqlite, no system openssl/sqlite) with the modern
# libfuzzer-sys cargo-fuzz 0.12 expects. Keeping it separate leaves upstream files untouched
# (the integration stays purely additive). cargo-fuzz targets it via `--fuzz-dir mayhem/fuzz`.
#
# cargo-fuzz drives the build:
#   - it provides its own libFuzzer runtime (the produced binary IS a libFuzzer target — Mayhem runs
#     it directly via `libfuzzer: true`, and it also runs once on a single input file as a reproducer);
#   - ASan is enabled the Rust way, through RUSTFLAGS `-Zsanitizer=address` (NOT clang's
#     $SANITIZER_FLAGS / CFLAGS — those don't apply to rustc), which is what OSS-Fuzz's `compile`
#     sets for FUZZING_LANGUAGE=rust. nightly is required for `-Zsanitizer`.
set -euo pipefail

# clang rejects SOURCE_DATE_EPOCH='' — must be unset or a valid integer (kept for parity even though
# the Rust build doesn't invoke clang directly; cargo's cc-built deps might).
[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH

: "${MAYHEM_JOBS:=$(nproc)}"
export MAYHEM_JOBS
# cargo-fuzz has no --jobs flag; cargo reads parallelism from CARGO_BUILD_JOBS.
export CARGO_BUILD_JOBS="$MAYHEM_JOBS"

# Debug info flags: must have DWARF < 4 symbols for Mayhem's coverage instrumentation.
# For Rust: -C debuginfo=2 for line tables + full debug info, plus LLVM flag for DWARF v3.
: "${RUST_DEBUG_FLAGS:=-C debuginfo=2 -C llvm-args=--dwarf-version=3}"
export RUST_DEBUG_FLAGS

cd "$SRC"

# Our additive cargo-fuzz crate. Discover every target from its fuzz_targets/ dir.
FUZZ_DIR="mayhem/fuzz"
FUZZ_TARGETS=()
for f in "$FUZZ_DIR"/fuzz_targets/*.rs; do
  FUZZ_TARGETS+=("$(basename "${f%.*}")")
done
[ "${#FUZZ_TARGETS[@]}" -gt 0 ] || { echo "ERROR: no fuzz targets under $FUZZ_DIR/fuzz_targets/" >&2; exit 1; }
TRIPLE="x86_64-unknown-linux-gnu"

# Replicate OSS-Fuzz `compile` RUSTFLAGS for a libFuzzer+ASan Rust build. cargo-fuzz sets the ASan
# flag itself by default, but we set it explicitly so the behavior is pinned and visible. `--cfg
# fuzzing` matches what libfuzzer-sys expects; force-frame-pointers aids ASan stack traces.
# Thread $RUST_DEBUG_FLAGS via LLVM for DWARF < 4 debug info (needed for Mayhem coverage).
export RUSTFLAGS="${RUSTFLAGS:-} --cfg fuzzing -Zsanitizer=address -Cdebuginfo=1 -Cforce-frame-pointers $RUST_DEBUG_FLAGS"

echo "=== cargo fuzz build (image-default nightly toolchain, ASan via RUSTFLAGS) ==="
echo "RUSTFLAGS=$RUSTFLAGS"
echo "targets: ${FUZZ_TARGETS[*]}"

# `-O` (release w/ opt) + `--debug-assertions` mirrors OSS-Fuzz's Rust build (catches overflow/debug
# asserts during fuzzing). Use the image's DEFAULT toolchain (Dockerfile pins it to the required
# nightly); a `+toolchain` override would make rustup try to install a different channel into the
# read-only shared /opt/rust. Build per-target so a single bad target doesn't mask the others.
for t in "${FUZZ_TARGETS[@]}"; do
  echo "--- building fuzz target: $t ---"
  cargo fuzz build --fuzz-dir "$FUZZ_DIR" -O --debug-assertions "$t"
  bin="$SRC/$FUZZ_DIR/target/$TRIPLE/release/$t"
  if [ ! -x "$bin" ]; then
    echo "ERROR: expected fuzz binary not found at $bin" >&2
    exit 1
  fi
  cp "$bin" "/mayhem/$t"
  echo "built /mayhem/$t"
done

echo "build.sh complete:"
ls -la "/mayhem/${FUZZ_TARGETS[@]}" 2>&1 || true
