# Ottex - Global Voice-to-Text for macOS

Ottex is a native macOS dictation app: press a global hotkey, speak, and the transcript is typed into the focused text field in any app. Speech-to-text runs on-device with Whisper (Metal-accelerated), using a Rust core and a SwiftUI frontend.

## Features

- **Global Dictation**: Press `⌥Space` from anywhere to start and stop recording
- **Paste Into Any Focused Text Field**: Transcribed text is inserted via macOS accessibility input simulation
- **On-Device Whisper**: Local speech-to-text powered by [`candle`](https://github.com/huggingface/candle) with Metal GPU acceleration
- **Background Utility App**: Runs as a lightweight agent-style app (`LSUIElement`)
- **Native SwiftUI UI**: Status, transcript, recent history, and first-run model selection

## Requirements

- macOS 13.0+
- Apple Silicon Mac (recommended)
- [Rust toolchain](https://rustup.rs/)
- Xcode Command Line Tools (`xcode-select --install`)
- Microphone permission
- Accessibility permission

## Quick Start

```bash
git clone https://github.com/Divhanthelion/app-ottex-macos.git
cd app-ottex-macos
./build_macos.sh
open Ottex.app
```

First launch downloads a Whisper model from Hugging Face (needs internet once). Then:

1. Leave Ottex running in the background
2. Focus any text field in any app
3. Press `⌥Space` to start recording
4. Press `⌥Space` again to stop and paste the transcript

Optional full clean rebuild:

```bash
CLEAN_BUILD=1 ./build_macos.sh
```

## Permissions

Ottex needs only two permissions:

- **Microphone** — capture speech
- **Accessibility** — insert transcribed text into the focused app

It does **not** require Input Monitoring or Apple Events for the current hotkey path.

## Hotkey

- `⌥Space` — toggle recording on/off globally

Registered via Carbon in `OttexSwiftUI/OttexSwiftUI/OttexEngineWrapper.swift`.

## Model Setup

On first run, choose a model tier:

- **Light** — faster, smaller (~150 MB download)
- **Standard** — balanced default
- **Maximum** — largest, highest accuracy

Models are cached locally after the first download.

## Project Layout

```
.
├── src/                         # Rust core (audio, Whisper/Metal, UniFFI)
├── OttexSwiftUI/OttexSwiftUI/   # SwiftUI app sources
├── build_macos.sh               # Builds Ottex.app
├── Ottex.entitlements
└── Cargo.toml
```

Generated at build time (not committed): `target/`, `bindings/`, `Ottex.app`.

## Rust-only checks

```bash
cargo build --release
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

## Troubleshooting

### Ottex does not type into other apps

- Grant Accessibility in System Settings → Privacy & Security → Accessibility
- Keep the target text field focused until transcription finishes
- If stuck, remove and re-add `Ottex.app` in Accessibility, then relaunch

### Global hotkey does not fire

- Confirm Ottex is still running
- Relaunch the signed `Ottex.app` from this directory

### No audio is captured

- Grant Microphone access in System Settings → Privacy & Security → Microphone
- Check the system input device

### Model download or load fails

- First run needs internet
- Ensure enough free disk space for the selected tier

## License

MIT License
