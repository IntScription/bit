#!/usr/bin/env bash
# Full pipeline: cross-compile bit-ffi for device + simulator, bundle it
# into an xcframework, regenerate Swift bindings, then build the BitApp
# Xcode project. Run this after any change to crates/bit-ffi or its
# dependencies. Requires the `aarch64-apple-ios` and `aarch64-apple-ios-sim`
# rustup targets (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim`).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "==> Cross-compiling bit-ffi for iOS device + simulator"
cargo build -p bit-ffi --release --target aarch64-apple-ios
cargo build -p bit-ffi --release --target aarch64-apple-ios-sim

echo "==> Regenerating Swift bindings"
./scripts/generate-swift-bindings.sh

echo "==> Preparing headers for xcframework"
mkdir -p ios/Headers
cp ios/Generated/bit_ffiFFI.h ios/Headers/
cp ios/Generated/bit_ffiFFI.modulemap ios/Headers/module.modulemap

echo "==> Building BitFFI.xcframework"
rm -rf ios/BitFFI.xcframework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libbit_ffi.a -headers ios/Headers \
  -library target/aarch64-apple-ios-sim/release/libbit_ffi.a -headers ios/Headers \
  -output ios/BitFFI.xcframework

echo "==> Building BitApp for the simulator"
# A concrete device destination, not 'generic/platform=iOS Simulator': the
# generic destination asks Xcode to build every arch that platform could
# mean, including x86_64 — which BitFFI.xcframework doesn't ship (only
# arm64 device + arm64 simulator; this repo targets Apple Silicon Macs).
# Picks the newest-runtime "iPhone 17"/"iPhone 16" available, not a fixed
# UDID — see run-ios-ui-tests.sh for why (a runtime older than the
# installed Xcode's Swift Testing support can crash at launch).
SIM_ID="${BIT_SIM_ID:-$(xcrun simctl list devices available -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
candidates = [
    (runtime, dev["name"] == "iPhone 17", dev["udid"])
    for runtime, devs in devices.items() if "iOS" in runtime
    for dev in devs if dev["name"] in ("iPhone 17", "iPhone 16")
]
candidates.sort(reverse=True)
print(candidates[0][2])
')}"
xcodebuild -project ios/BitApp/BitApp.xcodeproj -scheme BitApp \
  -sdk iphonesimulator -destination "id=$SIM_ID" build

echo "Done. Run with:"
echo "  xcrun simctl boot <device-id>              # if not already booted"
echo "  xcrun simctl install <device-id> <path-to>.app"
echo "  xcrun simctl launch <device-id> dev.kartiksanil.bit"
