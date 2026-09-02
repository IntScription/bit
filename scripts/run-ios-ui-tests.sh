#!/usr/bin/env bash
# Runs BitAppUITests against a booted (or bootable) simulator — real taps
# via Xcode's test runner, not host-level window automation (Simulator's
# window isn't exposed as a standard accessibility element, so System
# Events/AppleScript can't drive it reliably; XCUITest talks to the
# simulator directly and doesn't have that problem).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Picks the newest-runtime "iPhone 17"/"iPhone 16" available, not a fixed
# UDID: an iOS runtime older than the installed Xcode's own Swift Testing
# support can crash at launch with "Library not loaded:
# @rpath/lib_TestingInterop.dylib" (hit once against an iOS 18.5 sim while
# Xcode 26 was installed) — always prefer the latest paired runtime.
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

xcodebuild test -project ios/BitApp/BitApp.xcodeproj -scheme BitApp \
  -destination "id=$SIM_ID" -only-testing:BitAppUITests
