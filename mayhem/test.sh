#!/usr/bin/env bash
#
# meli/mayhem/test.sh — RUN meli's own functional test suite for the FUZZED library (`melib`) and
# emit a CTRF summary. exit 0 iff no test failed.
#
# PATCH-grade oracle: the fuzz target drives melib's RFC5322 e-mail/MIME parser
# (`melib::Envelope::from_bytes`). melib ships ~200 unit tests, including the parser/address/header
# suites (melib/src/email/parser/tests.rs, email/address/tests.rs, email/headers/tests.rs, ...) that
# assert BYTE/VALUE-EXACT parse results (decoded subjects, parsed addresses, MIME structure). A
# no-op / "exit(0)" / output-altering patch to the parser CANNOT pass these asserts. This script only
# RUNS the suite via `cargo test`; it never builds the fuzz target.
#
# We test melib with `--no-default-features` — the SAME minimal feature set the fuzz crate links
# (mayhem/fuzz: melib default-features=false). That keeps the oracle aligned with the fuzzed code,
# fast, and free of optional system deps (no imap/smtp/tls/sqlite → no openssl/sqlite needed).
#
# Scope: `--lib` (the crate's unit tests). These hold the parser/address/header assertion suites —
# the exact code the fuzz target exercises. We deliberately EXCLUDE doctests: several melib doctests
# (e.g. utils::patch_retrieve) reference feature-gated symbols (nntp/http) that don't exist under
# --no-default-features, so they fail to COMPILE for reasons unrelated to the fuzzed parser — they
# are not a functional signal here. Run with NORMAL flags (no sanitizer RUSTFLAGS) to keep the
# oracle honest and fast.
set -uo pipefail
[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH

: "${MAYHEM_JOBS:=$(nproc)}"
cd "$SRC"

# emit_ctrf <tool> <passed> <failed> [skipped] [pending] [other]
emit_ctrf() {
  local tool="$1" passed="$2" failed="$3" skipped="${4:-0}" pending="${5:-0}" other="${6:-0}"
  local tests=$(( passed + failed + skipped + pending + other ))
  cat > "${CTRF_REPORT:-$SRC/ctrf-report.json}" <<JSON
{
  "results": {
    "tool": { "name": "$tool" },
    "summary": {
      "tests": $tests,
      "passed": $passed,
      "failed": $failed,
      "pending": $pending,
      "skipped": $skipped,
      "other": $other
    }
  }
}
JSON
  printf 'CTRF {"results":{"tool":{"name":"%s"},"summary":{"tests":%d,"passed":%d,"failed":%d,"pending":%d,"skipped":%d,"other":%d}}}\n' \
    "$tool" "$tests" "$passed" "$failed" "$pending" "$skipped" "$other"
  [ "$failed" -eq 0 ]
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not available — cannot run the test suite" >&2
  emit_ctrf "cargo-test" 0 1 0; exit 2
fi

echo "=== running cargo test -p melib --no-default-features --lib (melib parser/unit suite) ==="
# Use the image's DEFAULT toolchain (the Dockerfile pins it to the same nightly the fuzz build uses),
# so no `+toolchain` override. --no-fail-fast so we count every test; RUSTFLAGS cleared so it inherits
# nothing from the sanitizer build.
out="$(RUSTFLAGS="" cargo test -p melib --no-default-features --lib --no-fail-fast --jobs "$MAYHEM_JOBS" 2>&1)"; rc=$?
echo "$out"

# libtest prints one line per test binary:
#   test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; ...
# Sum across all binaries.
PASSED=0; FAILED=0; IGNORED=0
while read -r p f i; do
  PASSED=$(( PASSED + p )); FAILED=$(( FAILED + f )); IGNORED=$(( IGNORED + i ))
done < <(printf '%s\n' "$out" \
  | sed -n 's/^test result:.* \([0-9][0-9]*\) passed; \([0-9][0-9]*\) failed; \([0-9][0-9]*\) ignored.*/\1 \2 \3/p')

# If we parsed no result lines, fall back to the cargo exit code (e.g. compile error).
if [ "$(( PASSED + FAILED + IGNORED ))" -eq 0 ]; then
  echo "could not parse any 'test result:' lines; using cargo exit code $rc" >&2
  [ "$rc" -eq 0 ] && { emit_ctrf "cargo-test" 1 0 0; exit 0; }
  emit_ctrf "cargo-test" 0 1 0; exit 1
fi

emit_ctrf "cargo-test" "$PASSED" "$FAILED" "$IGNORED"
