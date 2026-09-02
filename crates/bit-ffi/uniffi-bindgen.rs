//! Dev-only binary: generates Swift bindings from the compiled cdylib.
//! Not part of the app itself — see the `cli` feature in Cargo.toml.
//!
//! Usage (from the workspace root):
//!   cargo build -p bit-ffi --features bit-ffi/cli
//!   cargo run -p bit-ffi --bin uniffi-bindgen --features cli -- \
//!       generate --library target/debug/libbit_ffi.dylib --language swift --out-dir ios/Generated

fn main() {
    uniffi::uniffi_bindgen_main()
}
