# simpleSTT

A floating button that gives you instant speech-to-text on Windows. Press **F9**, speak, and your words are typed wherever your cursor is. No Electron, no frameworks, no bloat.

## Configuration

### First-Run Wizard

On first launch, simpleSTT walks you through:

1. **Model selection** — pick from installed `.bin` models in `models/`
2. **Microphone selection** — choose your input device
3. **Microphone preset** — choose a preset tuned for your setup

### Config File

Stored at `%APPDATA%\simplestt\simplestt\config\config.toml`:

```toml
[hotkeys]
start_stop = "F9"

[stt]
model_path = "models/ggml-base.bin"
language = "es"
beam_size = 5
patience = 1.0

[ui]
opacity = 220
size = 48

[audio]
microphone_only = true
preferred_input_device = ""
worker_sleep_ms = 10

[mic_preset]
name = "Headset / USB mic"
energy_threshold = 0.015
silence_frames_needed = 60
min_speech_samples = 8000
beam_size = 5
patience = 1.0
no_speech_thold = 0.6
entropy_thold = 2.4
```

### Settings Reference

| Setting | Default | Description |
|---------|---------|-------------|
| `hotkeys.start_stop` | `"F9"` | Global hotkey. Supports combos like `"Ctrl+F9"`, `"Alt+R"` |
| `stt.model_path` | Auto-detect | Path to whisper `.bin` model file |
| `stt.language` | `"es"` | Language code (`"es"`, `"en"`, `"fr"`, `"de"`, `"auto"`, etc.) |
| `stt.beam_size` | `5` | Beam search width (higher = more accurate, slower) |
| `stt.patience` | `1.0` | Beam search patience (0.0–2.0) |
| `ui.opacity` | `220` | Overlay opacity (0–255) |
| `ui.size` | `48` | Overlay circle size in pixels |
| `audio.microphone_only` | `true` | Skip loopback/system audio devices |
| `audio.preferred_input_device` | `""` | Partial name match for mic selection |
| `audio.worker_sleep_ms` | `10` | Audio polling interval in ms |

### Microphone Presets

| Preset | Best for | Energy threshold | Silence frames | Beam size |
|--------|----------|-----------------|----------------|-----------|
| Laptop built-in mic | Laptop mics, quiet room | 0.008 | 90 | 3 |
| Headset / USB mic | Headsets, USB mics | 0.015 | 60 | 5 |
| Studio / condenser mic | Professional mics | 0.025 | 50 | 7 |
| Noisy environment | Background noise | 0.035 | 100 | 5 |

The preset controls: `energy_threshold` (VAD sensitivity), `silence_frames_needed` (pause length before transcription), `min_speech_samples` (minimum audio to transcribe), `beam_size`, `patience`, `no_speech_thold`, and `entropy_thold`.

### Interactive Config Menu

```powershell
.\stt.ps1 config
```

Opens an interactive menu to change model, microphone preset, language, and hotkey without editing the TOML file directly. The model selector lists all 22 known whisper models with download-on-demand for uninstalled ones.

### Languages

Whisper supports 99 languages. Set `language` in config or via the config menu:

- `"es"` — Spanish
- `"en"` — English
- `"fr"` — French
- `"de"` — German
- `"auto"` — Auto-detect (less accurate)
- [Full list](https://github.com/openai/whisper/blob/main/whisper/tokenizer.py)

## License

MIT
