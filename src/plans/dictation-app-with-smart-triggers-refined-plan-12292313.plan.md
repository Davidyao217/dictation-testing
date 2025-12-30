## Objective

Build a minimal, hotkey-driven dictation app for Apple Silicon Macs. The app uses "smart triggers" to predictively load the Whisper model and prepare the audio pipeline, ensuring near-instant transcription when the hotkey is pressed.

---

## Architecture Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                        Smart Trigger Engine                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │ Cursor Move  │  │  Text Field  │  │   Keyboard Activity      │  │
│  │   Watcher    │  │ Focus Watcher│  │   (typing detected)      │  │
│  └──────┬───────┘  └──────┬───────┘  └────────────┬─────────────┘  │
│         └─────────────────┴───────────────────────┘                 │
│                            │                                        │
│                   ┌────────▼────────┐                               │
│                   │  Readiness      │                               │
│                   │  State Machine  │                               │
│                   └────────┬────────┘                               │
│         ┌──────────────────┼──────────────────┐                     │
│         ▼                  ▼                  ▼                     │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────────┐           │
│  │ Load Model  │   │ Open Audio  │   │ Pre-warm Model  │           │
│  │ to Memory   │   │   Stream    │   │ (dummy infer)   │           │
│  └─────────────┘   └─────────────┘   └─────────────────┘           │
└────────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────▼───────────────┐
              │      HOTKEY PRESSED           │
              │   (instant transcription)     │
              └───────────────────────────────┘
```

---

## Smart Trigger System

### Trigger Events (signals that dictation is likely)

| Trigger | Detection Method | Action |
| --- | --- | --- |
| **Cursor movement** | `CGEventTap` monitoring mouse/trackpad | Start loading model |
| **Text field focus** | Accessibility API (`AXFocusedUIElement`) | Open audio stream |
| **Keyboard activity** | Key event monitoring | Keep model warm |
| **App switch to text-heavy app** | Active app changes to Notes, browser, IDE, etc. | Full warmup |
| **Idle timeout** | No activity for 5+ minutes | Unload model to save RAM |

### Readiness States

```
COLD → LOADING → WARM → HOT → RECORDING → TRANSCRIBING
  │                       │
  └──────── (idle) ───────┘
```

| State | Description | Latency to Record |
| --- | --- | --- |
| `COLD` | Model not loaded, audio closed | \~2-3 seconds |
| `LOADING` | Model loading in background | \~1-2 seconds |
| `WARM` | Model loaded, audio stream closed | \~200ms |
| `HOT` | Model loaded + audio stream open + dummy inference done | &lt;50ms |
| `RECORDING` | Actively capturing audio | N/A |
| `TRANSCRIBING` | Processing audio through Whisper | N/A |

---

## Core Components

### 1. Smart Trigger Monitor

- **Location**: `src/triggers.rs`
- **Responsibilities**:
  - Monitor cursor movement via `CGEventTap`
  - Detect text field focus via Accessibility API
  - Track keyboard activity
  - Detect active application changes
  - Emit trigger events to state machine
- **Key APIs**: CoreGraphics `CGEventTap`, Accessibility `AXUIElement`

### 2. Readiness State Machine

- **Location**: `src/state.rs`
- **Responsibilities**:
  - Maintain current readiness level
  - Respond to trigger events by advancing state
  - Handle idle timeout to transition back to COLD
  - Coordinate model loader, audio manager, and warmup tasks
- **Transitions**: Event-driven, non-blocking

### 3. Model Manager

- **Location**: `src/model.rs`
- **Responsibilities**:
  - Lazy-load Whisper model on first trigger
  - Keep model in memory while in WARM/HOT state
  - Unload model on idle timeout
  - Run warmup inference (silent audio) to prime GPU caches
- **Key crate**: `whisper-rs`

### 4. Audio Manager

- **Location**: `src/audio.rs`
- **Responsibilities**:
  - Open/close microphone stream based on state
  - Ring buffer for continuous low-latency capture
  - Pre-open stream when entering HOT state
- **Key crate**: `cpal`

### 5. Hotkey Handler

- **Location**: `src/hotkey.rs`
- **Responsibilities**:
  - Register global hotkey (configurable)
  - Push-to-talk: record while held, transcribe on release
  - Coordinate with state machine for instant response
- **Key crate**: `global-hotkey`

### 6. Transcriber

- **Location**: `src/transcriber.rs`
- **Responsibilities**:
  - Run Whisper inference on captured audio
  - Metal acceleration for Apple Silicon
  - Return text result
- **Key crate**: `whisper-rs`

### 7. Output Handler

- **Location**: `src/output.rs`
- **Responsibilities**:
  - Write text to clipboard
  - Simulate Cmd+V paste to active app
- **Key crates**: `arboard`, `enigo`

### 8. Minimal Tray Icon

- **Location**: `src/tray.rs`
- **Responsibilities**:
  - Show status (idle/ready/recording/processing)
  - Quit menu item
  - Optional: settings shortcut
- **Key crates**: `tao`, `muda`

---

## File Structure

```
dictation_app/
├── Cargo.toml
├── build.rs                    # whisper-rs Metal compilation
├── src/
│   ├── main.rs                 # Entry point, event loop
│   ├── triggers.rs             # Smart trigger monitoring
│   ├── state.rs                # Readiness state machine
│   ├── model.rs                # Whisper model management
│   ├── audio.rs                # Microphone capture
│   ├── hotkey.rs               # Global hotkey
│   ├── transcriber.rs          # Whisper inference
│   ├── output.rs               # Clipboard + paste
│   ├── tray.rs                 # Menu bar icon
│   └── config.rs               # Settings (hotkey, model choice)
├── resources/
│   └── icons/                  # Status icons
└── Info.plist                  # Microphone + Accessibility permissions
```

---

## Implementation Phases

### Phase 1: Core Pipeline (Hotkey → Transcribe → Paste)

1. Set up Cargo project with dependencies
2. Implement basic hotkey registration
3. Implement audio capture (push-to-talk)
4. Integrate Whisper transcription
5. Implement clipboard paste output
6. **Verify**: End-to-end dictation works (cold start OK for now)

### Phase 2: State Machine & Model Management

1. Define readiness states enum
2. Implement state transitions
3. Add lazy model loading with background thread
4. Implement model warmup (dummy inference)
5. Add idle timeout to unload model
6. **Verify**: Model stays loaded between dictations

### Phase 3: Smart Triggers

1. Implement cursor movement detection via `CGEventTap`
2. Implement text field focus detection via Accessibility API
3. Implement keyboard activity monitoring
4. Implement active app change detection
5. Wire triggers to state machine
6. **Verify**: Model loads before hotkey is pressed

### Phase 4: Audio Stream Optimization

1. Pre-open audio stream in HOT state
2. Implement ring buffer for instant capture start
3. Add stream close on idle timeout
4. **Verify**: Audio latency &lt;50ms when HOT

### Phase 5: Polish & Reliability

1. Add minimal tray icon with status
2. Add config file for hotkey customization
3. Handle permission dialogs gracefully
4. Add proper error handling and logging
5. **Verify**: App runs reliably across restarts

---

## Key Dependencies

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
tokio = { version = "1", features = ["rt-multi-thread", "sync"] }
core-graphics = "0.24"         # CGEventTap for cursor/keyboard
accessibility = "0.1"          # AXUIElement for text field focus
```

---

## macOS Permissions Required

| Permission | Usage | Info.plist Key |
| --- | --- | --- |
| Microphone | Audio capture | `NSMicrophoneUsageDescription` |
| Accessibility | Text field focus detection, key simulation | System Preferences grant |
| Input Monitoring | Keyboard/cursor tracking | System Preferences grant |

---

## Verification / Definition of Done

| Requirement | Verification |
| --- | --- |
| Hotkey triggers recording | Press hotkey, see recording indicator |
| Smart trigger loads model | Move cursor, check logs for model load |
| Text field focus advances state | Click text box, verify HOT state |
| Transcription &lt;100ms when HOT | Stopwatch test from key-up to paste |
| Idle timeout unloads model | Wait 5 min, verify RAM usage drops |
| Paste works in any app | Test in Notes, Safari, VS Code |
| Tray icon shows status | Visual confirmation of state changes |

---

## Step → Targets → Verification

| Phase | Target Files | Verification |
| --- | --- | --- |
| 1 | `main.rs`, `hotkey.rs`, `audio.rs`, `transcriber.rs`, `output.rs` | End-to-end dictation works |
| 2 | `state.rs`, `model.rs` | Model persists between dictations |
| 3 | `triggers.rs`, `state.rs` | Model loads on cursor move |
| 4 | `audio.rs` | Audio latency &lt;50ms when HOT |
| 5 | `tray.rs`, `config.rs` | Tray shows status, config persists |

---

## Performance Targets

| Metric | Target | How Achieved |
| --- | --- | --- |
| Hotkey-to-record (HOT) | &lt;50ms | Pre-opened audio stream |
| Hotkey-to-record (COLD) | &lt;3s | Acceptable, smart triggers should prevent |
| Transcription speed | Real-time or faster | Metal acceleration, base.en model |
| Memory (model loaded) | \~300MB | whisper base.en model |
| Memory (idle/COLD) | &lt;50MB | Model unloaded after timeout |
