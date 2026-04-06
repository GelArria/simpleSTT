mod audio;
mod config;
mod hotkey;
mod injector;
mod overlay;
mod stt;

use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn needs_space_before(text: &str) -> bool {
    text.chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .count()
        == 0
}

const HOTKEY_ID: u32 = 1;

fn main() {
    simplelog::SimpleLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default())
        .expect("logger init failed");

    info!("simpleSTT starting...");

    let config = config::AppConfig::load();

    let stt = Arc::new(Mutex::new(None::<stt::SttEngine>));
    let recording = Arc::new(AtomicBool::new(false));

    match stt::SttEngine::new(
        &config.stt.model_path,
        &config.stt.language,
        config.stt.beam_size,
        config.stt.patience,
    ) {
        Ok(engine) => {
            *stt.lock().unwrap() = Some(engine);
            info!("STT engine loaded");
        }
        Err(e) => {
            error!("failed to load STT engine: {}", e);
            eprintln!("Error: failed to load whisper model from '{}': {}", config.stt.model_path, e);
            eprintln!("Download a ggml model to the 'models/' directory.");
            std::process::exit(1);
        }
    }

    let stt_clone = stt.clone();
    let recording_clone = recording.clone();
    let last_text = Arc::new(Mutex::new(String::new()));
    let last_text_clone = last_text.clone();
    let mic_only = config.audio.microphone_only;
    let preferred_input_device = if config.audio.preferred_input_device.trim().is_empty() {
        None
    } else {
        Some(config.audio.preferred_input_device.clone())
    };
    let worker_sleep_ms = config.audio.worker_sleep_ms.max(1);

    thread::spawn(move || {
        let mut audio_capture: Option<audio::AudioCapture> = None;
        let mut was_recording = false;

        loop {
            let is_recording = recording_clone.load(Ordering::SeqCst);

            if is_recording && !was_recording {
                info!("recording started - speak naturally, I'll transcribe after each pause");
                match audio::AudioCapture::new(16000, mic_only, preferred_input_device.clone()) {
                    Ok(capture) => {
                        if let Some(eng) = stt_clone.lock().unwrap().as_mut() {
                            eng.reset();
                        }
                        audio_capture = Some(capture);
                    }
                    Err(e) => error!("failed to start audio: {}", e),
                }
            }

            if !is_recording && was_recording {
                info!("recording stopped");
                if let Some(eng) = stt_clone.lock().unwrap().as_mut() {
                    if let Some(text) = eng.flush() {
                        info!("final transcript: {:?}", text);
                        let mut to_inject = String::new();
                        {
                            let last = last_text_clone.lock().unwrap();
                            if needs_space_before(&last) && !text.starts_with(char::is_whitespace) {
                                to_inject.push(' ');
                            }
                        }
                        to_inject.push_str(&text);
                        if let Err(e) = injector::inject_text(&to_inject) {
                            error!("inject failed: {}", e);
                        }
                        *last_text_clone.lock().unwrap() = to_inject;
                    }
                }
                audio_capture = None;
            }

            if is_recording {
                if let Some(capture) = &audio_capture {
                    let mut buf = Vec::new();
                    let n = capture.read_samples(&mut buf);
                    if n > 0 {
                        if let Some(eng) = stt_clone.lock().unwrap().as_mut() {
                            if let Some(text) = eng.process_audio(&buf) {
                                info!("utterance: {:?}", text);
                                let mut to_inject = String::new();
                                {
                                    let last = last_text_clone.lock().unwrap();
                                    if needs_space_before(&last) && !text.starts_with(char::is_whitespace) {
                                        to_inject.push(' ');
                                    }
                                }
                                to_inject.push_str(&text);
                                if let Err(e) = injector::inject_text(&to_inject) {
                                    error!("inject failed: {}", e);
                                }
                                *last_text_clone.lock().unwrap() = to_inject;
                            }
                        }
                    }
                }
            }

            was_recording = is_recording;
            thread::sleep(Duration::from_millis(worker_sleep_ms));
        }
    });

    let _hotkey = match hotkey::GlobalHotkey::register(&config.hotkeys.start_stop, HOTKEY_ID) {
        Ok(h) => {
            info!("hotkey registered: {}", config.hotkeys.start_stop);
            Some(h)
        }
        Err(e) => {
            warn!("failed to register hotkey '{}': {}", config.hotkeys.start_stop, e);
            None
        }
    };

    if let Err(e) = overlay::create_overlay(config.ui.opacity, config.ui.size, recording.clone()) {
        error!("failed to create overlay: {}", e);
        std::process::exit(1);
    }

    info!("simpleSTT ready - press {} to toggle, speak naturally, pauses trigger transcription", config.hotkeys.start_stop);

    let result = overlay::run_message_loop();

    if let Some(mut hk) = _hotkey {
        hk.unregister();
    }

    info!("simpleSTT exited with code {}", result);
}
