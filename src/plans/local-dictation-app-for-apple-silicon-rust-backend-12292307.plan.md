## Objective

Build a fast, responsive local dictation app for Apple Silicon Macs with a Rust backend. The app will capture audio on hotkey press, transcribe it locally using Whisper, and paste the result to the active application.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     macOS Menu Bar App                       │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐ │
│  │  Global Hotkey  │  │   System Tray   │  │   Settings   │ │
│  │    Listener     │  │   (Menu Bar)    │  │    Panel     │ │
│  └────────┬────────┘  └────────┬────────┘  └──────────────┘ │
│           │                    │                             │
│  ┌────────▼────────────────────▼────────────────────────────┐│
│  │                    Core Engine (Rust)                    ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  ││
│  │  │Audio Capture │  │   Whisper    │  │ Text Injection │  ││
│  │  │   (cpal)     │→ │ Transcriber  │→ │   (Paste)      │  ││
│  │  └──────────────┘  └──────────────┘  └────────────────┘  ││
│  └──────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

| Component | Library/Technology | Rationale |
| --- | --- | --- |
| **Speech-to-Text** | `whisper-rs` (whisper.cpp bindings) | Metal/CoreML acceleration on Apple Silicon, fully offline |
| **Audio Capture** | `cpal` | Cross-platform, CoreAudio backend for macOS, low latency |
| **Global Hotkeys** | `global-hotkey` | Maintained crate, macOS support, runs on main thread event loop |
| **UI Framework** | `tao` + `muda` (lightweight) | Native menu bar, minimal memory footprint |
| **Text Injection** | `enigo` or macOS CGEvent API | Simulate paste (Cmd+V) after clipboard write |
| **Clipboard** | `arboard` | Cross-platform clipboard access |
| **Config/Settings** | `serde` + `toml` | Human-readable config file |

---

## Core Components

### 1. Hotkey Manager

- **Location**: `src/hotkey.rs`
- **Responsibilities**:
  - Register global hotkey (default: `Cmd+Shift+D` or user-configurable)
  - Detect key-down (start recording) and key-up (stop recording) events
  - Support both push-to-talk and toggle modes
- **Key crate**: `global-hotkey`

### 2. Audio Capture Module

- **Location**: `src/audio.rs`
- **Responsibilities**:
  - Capture microphone input using CoreAudio backend
  - Buffer audio in memory (16kHz mono, f32 format for Whisper)
  - Handle audio device selection (default or user-specified)
- **Key crate**: `cpal`

### 3. Whisper Transcription Engine

- **Location**: `src/transcriber.rs`
- **Responsibilities**:
  - Load Whisper model (ggml format) - recommend `base.en` or `small.en` for speed
  - Run inference with Metal acceleration
  - Stream results or return full transcription
  - Support model selection via config
- **Key crate**: `whisper-rs`
- **Models directory**: `~/.dictation/models/`

### 4. Text Output Handler

- **Location**: `src/output.rs`
- **Responsibilities**:
  - Copy transcribed text to clipboard
  - Simulate Cmd+V to paste into active application
  - Optional: Direct text injection via Accessibility API
- **Key crates**: `arboard`, `enigo`

### 5. Menu Bar Interface

- **Location**: `src/ui/tray.rs`
- **Responsibilities**:
  - Display recording status indicator (icon changes)
  - Provide quick access to settings
  - Show recent transcriptions (optional)
- **Key crates**: `tao`, `muda`

### 6. Configuration System

- **Location**: `src/config.rs`
- **Responsibilities**:
  - Load/save user preferences
  - Configurable hotkey, model selection, audio device
- **Config file**: `~/.dictation/config.toml`

---

## File Structure

```
dictation_app/
├── Cargo.toml
├── build.rs                    # Build script for whisper-rs
├── src/
│   ├── main.rs                 # Entry point, event loop
│   ├── app.rs                  # App state management
│   ├── hotkey.rs               # Global hotkey handling
│   ├── audio.rs                # Microphone capture
│   ├── transcriber.rs          # Whisper integration
│   ├── output.rs               # Clipboard + paste
│   ├── config.rs               # Settings management
│   └── ui/
│       ├── mod.rs
│       └── tray.rs             # Menu bar interface
├── resources/
│   └── icons/                  # Menu bar icons (idle, recording, processing)
└── Info.plist                  # macOS app metadata (microphone permission)
```

---

## Implementation Steps

### Phase 1: Project Setup & Core Infrastructure

1. Initialize Cargo project with workspace structure
2. Add dependencies to `Cargo.toml`
3. Create `build.rs` for whisper-rs compilation with Metal support
4. Set up basic app skeleton with `tao` event loop

### Phase 2: Audio Pipeline

1. Implement microphone capture with `cpal`
2. Add audio buffering (ring buffer for continuous capture)
3. Implement resampling to 16kHz if needed
4. Test audio capture independently

### Phase 3: Whisper Integration

1. Implement model loading and caching
2. Add transcription function with proper audio format conversion
3. Enable Metal acceleration flags
4. Test transcription with recorded audio files

### Phase 4: Hotkey System

1. Implement global hotkey registration
2. Add push-to-talk mode (record while held)
3. Add toggle mode option (press to start/stop)
4. Integrate hotkey events with audio capture

### Phase 5: Text Output

1. Implement clipboard writing
2. Add simulated paste (Cmd+V)
3. Handle edge cases (no active text field, etc.)

### Phase 6: Menu Bar UI

1. Create system tray icon with status indicators
2. Add context menu (Settings, Quit, etc.)
3. Implement settings panel (basic)

### Phase 7: Configuration & Polish

1. Implement config file loading/saving
2. Add model download helper (or document manual download)
3. Add proper error handling and logging
4. Test end-to-end workflow

### Phase 8: macOS Integration

1. Create `Info.plist` with microphone usage description
2. Add proper app icons
3. Configure code signing (for distribution)
4. Test Accessibility permissions for text injection

---

## Key Configuration Options

```toml
# ~/.dictation/config.toml

[hotkey]
modifier = "cmd+shift"
key = "d"
mode = "push_to_talk"  # or "toggle"

[audio]
device = "default"
sample_rate = 16000

[model]
name = "base.en"       # tiny.en, base.en, small.en, medium.en
path = "~/.dictation/models/"

[output]
auto_paste = true
add_space_after = true
```

---

## Performance Optimizations (Apple Silicon Focus)

1. **Metal Acceleration**: Enable via `whisper-rs` feature flags
2. **Model Selection**: Use `base.en` (74MB) for balance of speed/accuracy
3. **Lazy Model Loading**: Load model on first use, keep in memory
4. **Audio Buffer**: Use lock-free ring buffer to prevent hotkey lag
5. **Async Processing**: Transcribe in background thread, don't block main loop
6. **Warm-up**: Run dummy inference on startup for faster first transcription

---

## Dependencies (Cargo.toml)

```toml
[dependencies]
whisper-rs = { version = "0.14", features = ["metal"] }
cpal = "0.15"
global-hotkey = "0.7"
tao = "0.32"
muda = "0.17"
arboard = "3.4"
enigo = "0.3"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
anyhow = "1.0"
log = "0.4"
env_logger = "0.11"
tokio = { version = "1", features = ["rt-multi-thread", "sync"] }
```

---

## Verification / Definition of Done

| Requirement | Verification Method |
| --- | --- |
| Hotkey responds in &lt;50ms | Manual latency testing with timer |
| Audio capture works | Test recording saves valid WAV file |
| Whisper transcribes correctly | Compare output to known audio samples |
| Metal acceleration active | Check `whisper-rs` logs for Metal backend |
| Text pastes to active app | Test in various apps (Notes, browser, IDE) |
| Menu bar icon appears | Visual confirmation, icon state changes |
| Config loads/saves | Modify config, restart, verify persistence |
| Microphone permission prompt | First launch triggers macOS permission dialog |

---

## Step-to-Target Traceability

| Step | Target Files | Verification |
| --- | --- | --- |
| Phase 1 | `Cargo.toml`, `src/main.rs`, `build.rs` | `cargo build` succeeds |
| Phase 2 | `src/audio.rs` | Audio test records to buffer |
| Phase 3 | `src/transcriber.rs` | Transcription test passes |
| Phase 4 | `src/hotkey.rs`, `src/main.rs` | Hotkey triggers callback |
| Phase 5 | `src/output.rs` | Text appears in Notes.app |
| Phase 6 | `src/ui/tray.rs` | Icon visible in menu bar |
| Phase 7 | `src/config.rs` | Config persists across restarts |
| Phase 8 | `Info.plist`, resources | App signed and runs from /Applications |

---

## Risks & Mitigations

| Risk | Mitigation |
| --- | --- |
| Microphone permission denied | Clear Info.plist usage description, graceful error |
| Whisper model too slow | Default to `tiny.en`, let user upgrade |
| Metal not available | Fallback to CPU with warning |
| Hotkey conflict with other apps | Allow user to customize hotkey |
| Large model download | Provide download script or first-run wizard |
