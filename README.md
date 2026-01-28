# Ottex - Voice-to-Text for Windows

A Windows application that provides voice-to-text functionality with AI-powered text shortcuts, built in Rust.

## Features

- **Voice-to-Text**: Press `Alt+Space` to start recording, release to transcribe and type
- **AI Shortcuts**: Press `Ctrl+Shift+R` to apply "Fix Grammar" to selected text
- **System Tray**: Runs quietly in the background with tray icon access
- **Multiple Providers**: Supports Google Speech-to-Text, OpenAI Whisper, and OpenRouter

## Installation

### Prerequisites

- Windows 10/11
- Rust toolchain (for building from source)

### Building from Source

```bash
cargo build --release
```

The executable will be at `target/release/ottex.exe`.

## Configuration

On first run, a configuration file is created at:
```
%APPDATA%\ottex\Ottex\config.toml
```

You can access it via the "Settings..." menu in the system tray.

### Configuration Options

```toml
[transcription]
# Provider: "google", "openai", or "openrouter"
provider = "openai"

# API Keys (set at least one)
google_api_key = ""
openai_api_key = "sk-..."
openrouter_api_key = ""

# Language for transcription
language = "en-US"

[hotkeys]
# Hotkey for voice recording
record = "Alt+Space"
# Hotkey for AI shortcuts
ai_shortcut = "Ctrl+Shift+R"

[ai_shortcuts]
# Provider for AI text shortcuts: "openai" or "openrouter"
provider = "openai"
# Model to use
model = "gpt-4o-mini"
```

## Usage

1. **Start the app**: Run `ottex.exe` - it will appear in the system tray
2. **Voice Recording**: Hold `Alt+Space`, speak, then release to transcribe
3. **AI Shortcuts**: Select text in any app, press `Ctrl+Shift+R` to fix grammar
4. **Settings**: Right-click tray icon and select "Settings..." to edit config
5. **Quit**: Right-click tray icon and select "Quit"

## API Setup

### OpenAI (Recommended)

1. Get an API key from https://platform.openai.com/api-keys
2. Add to config: `openai_api_key = "sk-..."`

### Google Speech-to-Text

1. Create a Google Cloud project
2. Enable Speech-to-Text API
3. Create an API key
4. Add to config: `google_api_key = "..."`

### OpenRouter

1. Get an API key from https://openrouter.ai/keys
2. Add to config: `openrouter_api_key = "sk-or-..."`

## Troubleshooting

### Hotkeys not working
- Make sure no other app is using the same hotkey combination
- Try running as Administrator if targeting admin-level windows

### Typing not working
- Don't run the app as Administrator unless necessary (UIPI restrictions)
- The target window must be focused when transcription completes

### No audio captured
- Check Windows microphone permissions for the app
- Verify microphone is set as default input device

## Tech Stack

- **tray-icon**: System tray integration
- **global-hotkey**: Global hotkey registration
- **cpal**: Cross-platform audio capture
- **hound**: WAV encoding
- **reqwest**: HTTP client for API calls
- **enigo**: Keyboard/mouse input simulation
- **arboard**: Clipboard access

## License

MIT License
