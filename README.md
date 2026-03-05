# Ottex - Global Voice-to-Text for macOS

Ottex is a native macOS dictation app that lets you press a global hotkey, speak, and drop the transcribed text into the currently focused text field in any app. It uses on-device Whisper inference with Metal acceleration, a Rust core, and a SwiftUI frontend.

## Features

- **Global Dictation**: Press `⌥Space` from anywhere to start and stop recording
- **Paste Into Any Focused Text Field**: Transcribed text is inserted into the active app using macOS accessibility input simulation
- **On-Device Whisper**: Local speech-to-text powered by [`candle`](https://github.com/huggingface/candle) with Metal GPU acceleration
- **Background Utility App**: Runs as a lightweight macOS agent-style app with [`LSUIElement`](app-ottex/build_macos.sh:59)
- **Ambient Native UI**: Redesigned SwiftUI interface with an animated mesh-style background, hero status panel, transcript panel, and recent history
- **Model Selection on First Run**: Choose a lighter or larger Whisper model tier depending on speed vs. accuracy
- **Optional Provider Plumbing**: The Rust core still contains support scaffolding for LM Studio, OpenAI, Google, and OpenRouter workflows

## Requirements

- macOS 13.0+
- Apple Silicon Mac recommended
- Rust toolchain
- Microphone permission
- Accessibility permission

## Permissions Required

Ottex currently needs only two macOS permissions:

- **Microphone**: to capture speech
- **Accessibility**: to insert transcribed text into the focused app

Ottex does **not** currently require Input Monitoring or Apple Events for the implemented global hotkey path.

## Build

The project consists of the Rust library in [`app-ottex`](app-ottex) and the SwiftUI frontend in [`OttexSwiftUI`](OttexSwiftUI). Build the app bundle from the [`app-ottex`](app-ottex) directory:

```bash
./build_macos.sh
open Ottex.app
```

For Rust-only work:

```bash
cargo build --release
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

## How It Works

1. Launch [`Ottex.app`](app-ottex/Ottex.app)
2. Leave it running in the background
3. Focus any text input in any macOS app
4. Press `⌥Space` to start recording
5. Press `⌥Space` again to stop and transcribe
6. Ottex pastes the resulting text into the focused field

The in-app interface also lets you:

- review the latest transcript
- copy or export text
- view recent captures
- pick a model tier on first launch

## Current Hotkey Behavior

The current macOS implementation uses a Carbon global hotkey registered in [`GlobalHotkeyManager`](OttexSwiftUI/OttexSwiftUI/OttexEngineWrapper.swift:19). The active hotkey is:

- `⌥Space` — toggle recording on/off globally

While the Rust config still contains generalized hotkey fields, the shipping macOS UI currently uses the hardcoded Carbon path above.

## Model Setup

On first run, Ottex asks you to choose a model tier:

- **Light** — faster, smaller, lower accuracy
- **Standard** — balanced default
- **Maximum** — largest, highest accuracy

The selected tier maps to a Whisper repo and is loaded by [`OttexEngineWrapper`](OttexSwiftUI/OttexSwiftUI/OttexEngineWrapper.swift:155).

Model files are downloaded from Hugging Face and cached locally on first use.

## Configuration Notes

The Rust core still includes a broader configuration model in [`config.rs`](app-ottex/src/config.rs), including:

- embedded Whisper settings
- local server URL support
- cloud provider API key fields
- AI shortcut settings

That configuration layer is not yet fully surfaced in the current SwiftUI product flow. For a public release, treat the current app as:

- **primary path**: embedded Whisper on-device transcription
- **primary UX**: global `⌥Space` dictation

## Troubleshooting

### Ottex does not type into other apps

- Grant Accessibility access in System Settings → Privacy & Security → Accessibility
- Make sure the target app still has the text field focused when transcription finishes
- If permission state seems stale, remove and re-add [`Ottex.app`](app-ottex/Ottex.app) in Accessibility and relaunch it

### Global hotkey does not fire

- Make sure Ottex is still running in the background
- `⌥Space` is registered through Carbon and should work without Input Monitoring
- Relaunch the rebuilt signed bundle from [`app-ottex/Ottex.app`](app-ottex/Ottex.app)

### No audio is captured

- Grant Microphone access in System Settings → Privacy & Security → Microphone
- Check that the correct microphone is selected as the system input device

### Model download or load fails

- First run needs internet access to download model files
- Ensure enough free disk space for the selected model tier
- Cached model files live under the Ottex cache directory managed by [`ProjectDirs`](app-ottex/src/config.rs:217)

## Architecture

### Rust Core

The Rust core in [`app-ottex/src`](app-ottex/src) handles:

- audio capture via [`cpal`](app-ottex/Cargo.toml:11)
- resampling via [`rubato`](app-ottex/Cargo.toml:46)
- Whisper inference via [`candle-core`](app-ottex/Cargo.toml:39), [`candle-nn`](app-ottex/Cargo.toml:40), and [`candle-transformers`](app-ottex/Cargo.toml:41)
- text insertion via [`enigo`](app-ottex/Cargo.toml:19)
- UniFFI bindings for Swift integration

### SwiftUI Frontend

The SwiftUI app in [`OttexSwiftUI`](OttexSwiftUI) provides:

- the redesigned ambient two-panel interface in [`ContentView`](OttexSwiftUI/OttexSwiftUI/ContentView.swift:272)
- first-run model selection in [`ModelSetupView`](OttexSwiftUI/OttexSwiftUI/ContentView.swift:624)
- the Carbon-based global hotkey manager in [`OttexEngineWrapper.swift`](OttexSwiftUI/OttexSwiftUI/OttexEngineWrapper.swift)

## Repo Hygiene for Public GitHub Publishing

Do not publish generated/local artifacts such as:

- [`target/`](app-ottex/target)
- [`Ottex.app`](app-ottex/Ottex.app)
- generated files in [`bindings/`](app-ottex/bindings)
- local planning docs or dumps

Those are already covered by the updated [`.gitignore`](app-ottex/.gitignore).

## License

MIT License
