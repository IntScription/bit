#!/usr/bin/env bash
# Regenerates ios/Generated/* from bit-ffi's current #[uniffi::export] surface.
# Run this after any change to crates/bit-ffi/src/lib.rs — the generated
# files are checked in (see /ARCHITECTURE.md §12, risk 7) so the Xcode
# project can reference a stable path, but they are never hand-edited.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo build -p bit-ffi
cargo run -p bit-ffi --bin uniffi-bindgen --features cli -- \
    generate --library target/debug/libbit_ffi.dylib --language swift --out-dir ios/Generated

echo "Swift bindings regenerated in ios/Generated/"
