# simpleSTT

A single floating button that wraps [whisper.cpp](https://github.com/ggml-org/whisper.cpp) (via [whisper-rs](https://github.com/tazz4843/whisper-rs)) to give you instant speech-to-text on Windows.

Click the button or press **F9** → speak → your words are typed wherever your cursor is. That's it.

No Electron, no frameworks, no bloat. ~3 MB binary, GPU-accelerated.

## What It Does

Press **F9** (or click the overlay button) to start recording. Speak naturally. When you pause, whisper.cpp transcribes the audio and the text is typed directly into whatever field is focused — your browser, IDE, chat app, anything.

The overlay is a small draggable circle that sits on top of everything:
- **Gray** = idle
- **Red pulsing** = recording

## Requirements

- **Windows 10/11** (x64)
- **Rust** (latest stable) — [install](https://rustup.rs/)
- **Visual Studio Build Tools** with C++ workload
- **LLVM** (for bindgen) — `winget install LLVM.LLVM`
- **NVIDIA GPU + CUDA Toolkit** (recommended) — or use CPU/Vulkan

## Quick Start

### 1. Clone and download a model

```powershell
git clone https://github.com/your-username/simpleSTT.git
cd simpleSTT
mkdir models
Invoke-WebRequest -Uri "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin" -OutFile "models/ggml-large-v3-turbo.bin"
```

Models (best to worst accuracy):

| Model | Size | Accuracy | Notes |
|-------|------|----------|-------|
| `ggml-large-v3-turbo.bin` | ~800 MB | Best | Default, recommended |
| `ggml-medium.bin` | ~1.5 GB | Very good | Slower |
| `ggml-base.bin` | ~140 MB | Good | Fast |
| `ggml-tiny.bin` | ~75 MB | Basic | Quick testing |

### 2. Build

**With CUDA (NVIDIA GPU — recommended):**

```powershell
cargo build --release --features cuda
```

**CPU only:**

```powershell
cargo build --release
```

**With Vulkan (AMD/Intel GPU):**

```powershell
cargo build --release --features vulkan
```

### 3. Run

```powershell
.\target\release\simplestt.exe
```

Or use the management script:

```powershell
.\stt.ps1 run        # foreground with logs
.\stt.ps1 start      # start in background
.\stt.ps1 stop       # stop
.\stt.ps1 restart    # restart
.\stt.ps1 status     # check if running
.\stt.ps1 install    # build + create Start Menu shortcut
.\stt.ps1 uninstall  # stop + remove shortcuts
```

## Usage

| Action | How |
|--------|-----|
| Toggle recording | Press **F9** or click the overlay |
| Move overlay | Click and drag the circle |
| Context menu | Right-click the overlay |
| Quit | Right-click → Quit |

## Configuration

Auto-generated on first run at `%APPDATA%\simplestt\simplestt\config\config.toml`:

```toml
[hotkeys]
start_stop = "F9"

[stt]
model_path = "models/ggml-large-v3-turbo.bin"
language = "es"
beam_size = 5
patience = 1.0

[ui]
opacity = 180
size = 40

[audio]
microphone_only = true
preferred_input_device = ""
worker_sleep_ms = 10
```

| Setting | Default | Description |
|---------|---------|-------------|
| `hotkeys.start_stop` | `"F9"` | Global hotkey. Supports combos like `"Ctrl+F9"` |
| `stt.model_path` | `"models/ggml-large-v3-turbo.bin"` | Path to whisper model file |
| `stt.language` | `"es"` | Language code (`"es"`, `"en"`, `"auto"`, etc.) |
| `stt.beam_size` | `5` | Beam search width (higher = more accurate, slower) |
| `stt.patience` | `1.0` | Beam search patience (0.0–2.0) |
| `ui.opacity` | `180` | Overlay opacity (0–255) |
| `ui.size` | `40` | Overlay circle size in pixels |
| `audio.microphone_only` | `true` | Skip loopback/system audio devices |
| `audio.preferred_input_device` | `""` | Partial name match for mic selection |
| `audio.worker_sleep_ms` | `10` | Audio polling interval in ms |

## Languages

Whisper supports 99 languages. Set `language` in config:

- `"es"` — Spanish
- `"en"` — English
- `"fr"` — French
- `"de"` — German
- `"auto"` — Auto-detect (less accurate)
- [Full list](https://github.com/openai/whisper/blob/main/whisper/tokenizer.py)

## How It Works Under the Hood

```
[F9 / Click] → cpal captures mic audio at 16 kHz
             → VAD detects speech vs silence
             → On pause (~0.7s silence), whisper.cpp transcribes via whisper-rs
             → SendInput types the text into the focused window
```

```
src/
  main.rs      — Entry point, worker thread, pipeline
  stt.rs       — Whisper engine (whisper-rs), VAD, transcription
  audio.rs     — Mic capture + resampling (cpal + ringbuf)
  injector.rs  — Text injection via SendInput (Unicode)
  hotkey.rs    — Global hotkey registration (Win32)
  overlay.rs   — Draggable overlay button (Win32 + GDI)
  config.rs    — TOML config load/save
```

## Building from Scratch (Troubleshooting)

If the build fails with bindgen errors, set include paths:

```powershell
$env:BINDGEN_EXTRA_CLANG_ARGS = "-I`"C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\ucrt`" -I`"C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um`" -I`"C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\shared`" -I`"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.xx.xxxxx\include`""
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\LLVM\bin;$env:PATH"

cargo build --release --features cuda
```

## License

MIT
