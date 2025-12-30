# Local Dictation Tool

A simple, local voice dictation tool built in Rust. It listens for a global hotkey, records your voice, transcribes it using OpenAI's Whisper (running locally), and types the text into your active window.

## Context
This was a fun pet project created to familiarize myself with modern AI-assisted coding tools. The entire codebase was built with the help of:
- **Antigravity**
- **Verdant**
- **Opencoder**

The 2 goals were: 1. latency and 2. efficiency


## Inspiration
Heavily inspired by [handy](https://github.com/handy/handy). I wanted to see if I could build a lightweight version of that workflow from scratch using AI agents. 

## Features
- **Global Hotkey**: Press `Cmd+Shift+D` to start/stop recording.
- **Local Transcription**: Uses `whisper-rs` to run models locally (no API keys or cloud data).
- **Modes**: Supports both "Push-to-Talk" and "Toggle" recording modes.
- **Visual Feedback**: A minimal on-screen indicator shows when it's recording or processing.
- **Smart Output**: Automatically types the transcribed text into your active text field.

## Getting Started
1. **Prerequisites**: Ensure you have Rust installed.
2. **Download Model**: 
   Run the included script to download the base Whisper model:
   ```bash
   ./download_model.sh
   ```
   This will place the model in `~/.dictation/models/`.
3. **Run**:
   ```bash
   cargo run --release
   ```

## Configuration
On first run, a config file is created at `~/.dictation/config.toml`. You can edit this to change:
- `recording_mode`: "push_to_talk" (default) or "toggle"
- `vnad_enabled` / `vad_threshold`: Voice activity detection settings
- `model`: Change which model size to use

## Development
This project is written in Rust and uses:
- `cpal` for audio input
- `whisper-rs` for inference
- `enigo` for text injection
- `tao` / `muda` for system tray and windowing management

## License
MIT / Open usage. Use it however you want.
