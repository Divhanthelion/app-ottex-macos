#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

# Keep build artifacts inside this repo (ignore any inherited CARGO_TARGET_DIR).
unset CARGO_TARGET_DIR
export CARGO_TARGET_DIR="$ROOT_DIR/target"

export MACOSX_DEPLOYMENT_TARGET=13.0
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-mmacosx-version-min=13.0"

if [[ "${CLEAN_BUILD:-0}" == "1" ]]; then
  echo "=> Cleaning Rust build artifacts..."
  cargo clean
fi

echo "=> Building Rust Core (Metal + Uniffi)..."
cargo build --release

echo "=> Generating Swift Bindings..."
mkdir -p bindings
cargo run --bin uniffi-bindgen -- generate src/ottex.udl --language swift --out-dir bindings

echo "=> Compiling SwiftUI Application natively without Xcode..."
swiftc -o OttexApp \
    -target arm64-apple-macos13.0 \
    -import-objc-header bindings/ottexFFI.h \
    bindings/ottex.swift \
    OttexSwiftUI/OttexSwiftUI/OttexSwiftUIApp.swift \
    OttexSwiftUI/OttexSwiftUI/OttexEngineWrapper.swift \
    OttexSwiftUI/OttexSwiftUI/ContentView.swift \
    target/release/libottex.a \
    -framework AudioUnit \
    -framework SystemConfiguration \
    -framework CoreAudio \
    -framework Security \
    -framework Foundation \
    -framework AppKit \
    -framework Accelerate \
    -framework Metal \
    -framework MetalPerformanceShaders \
    -framework Carbon \
    -framework CoreGraphics \
    -framework ApplicationServices \
    -lc++ \
    -O

echo "=> Assembling standard macOS .app Bundle..."
rm -rf Ottex.app
mkdir -p Ottex.app/Contents/MacOS
mkdir -p Ottex.app/Contents/Resources

# Move the executable
mv OttexApp Ottex.app/Contents/MacOS/Ottex

# Generate a minimal Info.plist
cat > Ottex.app/Contents/Info.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key>
	<string>Ottex</string>
	<key>CFBundleIdentifier</key>
	<string>com.rj.ottex</string>
	<key>CFBundleName</key>
	<string>Ottex</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>LSMinimumSystemVersion</key>
	<string>13.0</string>
	<key>LSUIElement</key>
	<true/>
    <key>NSMicrophoneUsageDescription</key>
    <string>Ottex needs access to your microphone for Whisper Voice-to-Text transcription.</string>
    <key>NSAccessibilityUsageDescription</key>
    <string>Ottex requires accessibility access to simulate keyboard typing for transcribed text.</string>
</dict>
</plist>
EOF

echo "=> Code signing the application..."
codesign --force --deep --options runtime --entitlements Ottex.entitlements --sign - Ottex.app

echo "✅ Success! Ottex.app has been created in the current directory."
echo "You can launch it by running: open Ottex.app"
