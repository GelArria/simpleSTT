use cpal::{traits::*, Stream};
use log::{error, info, warn};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
pub struct AudioCapture {
    _stream: Option<Stream>,
    producer: Arc<Mutex<ringbuf::HeapProd<f32>>>,
    consumer: Arc<Mutex<ringbuf::HeapCons<f32>>>,
    target_rate: u32,
    active: bool,
    microphone_only: bool,
    preferred_input_device: Option<String>,
}

#[allow(dead_code)]
impl AudioCapture {
    pub fn new(
        target_rate: u32,
        microphone_only: bool,
        preferred_input_device: Option<String>,
    ) -> Result<Self, String> {
        let rb = HeapRb::new(target_rate as usize * 10);
        let (prod, cons) = rb.split();
        let producer = Arc::new(Mutex::new(prod));
        let consumer = Arc::new(Mutex::new(cons));

        let host = cpal::default_host();
        let device =
            select_input_device(&host, microphone_only, preferred_input_device.as_deref())?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("input config error: {}", e))?;

        info!(
            "input: {}, rate: {:?}, ch: {}, fmt: {:?}",
            device.name().unwrap_or_default(),
            config.sample_rate(),
            config.channels(),
            config.sample_format()
        );

        let src_rate = config.sample_rate().0;
        let src_channels = config.channels();
        let sample_format = config.sample_format();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_stream_f32(
                &device,
                &config.into(),
                src_rate,
                src_channels,
                target_rate,
                producer.clone(),
            )?,
            cpal::SampleFormat::I16 => build_stream_i16(
                &device,
                &config.into(),
                src_rate,
                src_channels,
                target_rate,
                producer.clone(),
            )?,
            cpal::SampleFormat::U16 => build_stream_u16(
                &device,
                &config.into(),
                src_rate,
                src_channels,
                target_rate,
                producer.clone(),
            )?,
            _ => return Err(format!("unsupported format: {:?}", sample_format)),
        };

        Ok(Self {
            _stream: Some(stream),
            producer,
            consumer,
            target_rate,
            active: true,
            microphone_only,
            preferred_input_device,
        })
    }

    pub fn read_samples(&self, buf: &mut Vec<f32>) -> usize {
        let start = buf.len();
        if let Ok(mut guard) = self.consumer.lock() {
            while let Some(s) = guard.try_pop() {
                buf.push(s);
            }
        }
        buf.len() - start
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn stop(&mut self) {
        self._stream = None;
        self.active = false;
        info!("audio stopped");
    }

    pub fn resume(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = select_input_device(
            &host,
            self.microphone_only,
            self.preferred_input_device.as_deref(),
        )?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("input config error: {}", e))?;

        let src_rate = config.sample_rate().0;
        let src_channels = config.channels();
        let sample_format = config.sample_format();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_stream_f32(
                &device,
                &config.into(),
                src_rate,
                src_channels,
                self.target_rate,
                self.producer.clone(),
            )?,
            cpal::SampleFormat::I16 => build_stream_i16(
                &device,
                &config.into(),
                src_rate,
                src_channels,
                self.target_rate,
                self.producer.clone(),
            )?,
            cpal::SampleFormat::U16 => build_stream_u16(
                &device,
                &config.into(),
                src_rate,
                src_channels,
                self.target_rate,
                self.producer.clone(),
            )?,
            _ => return Err(format!("unsupported format: {:?}", sample_format)),
        };

        self._stream = Some(stream);
        self.active = true;
        info!("audio resumed");
        Ok(())
    }
}

pub fn looks_like_loopback(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("stereo mix")
        || lower.contains("what u hear")
        || lower.contains("loopback")
        || lower.contains("wave out")
        || lower.contains("mix")
}

fn select_input_device(
    host: &cpal::Host,
    microphone_only: bool,
    preferred_input_device: Option<&str>,
) -> Result<cpal::Device, String> {
    let preferred = preferred_input_device
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase());

    let mut fallback: Option<cpal::Device> = None;

    let devices = host
        .input_devices()
        .map_err(|e| format!("failed to enumerate input devices: {}", e))?;

    for device in devices {
        let name = device
            .name()
            .unwrap_or_else(|_| String::from("unknown input"));
        let loopback = looks_like_loopback(&name);

        if loopback {
            warn!("skipping loopback-like input device: {}", name);
            if fallback.is_none() {
                fallback = Some(device);
            }
            continue;
        }

        if let Some(pref) = preferred.as_deref() {
            if name.to_lowercase().contains(pref) {
                info!("selected preferred input device: {}", name);
                return Ok(device);
            }
        } else {
            info!("selected input device: {}", name);
            return Ok(device);
        }
    }

    if microphone_only {
        return Err(
            "no valid microphone input found (only loopback-like devices were detected)"
                .to_string(),
        );
    }

    if let Some(device) = fallback {
        let name = device
            .name()
            .unwrap_or_else(|_| String::from("unknown input"));
        warn!("falling back to loopback-like input device: {}", name);
        return Ok(device);
    }

    host.default_input_device()
        .ok_or_else(|| "no input device found".to_string())
}

fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    src_rate: u32,
    src_channels: u16,
    target_rate: u32,
    producer: Arc<Mutex<ringbuf::HeapProd<f32>>>,
) -> Result<Stream, String> {
    let ratio = target_rate as f64 / src_rate as f64;

    let stream = device
        .build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let input_frames = data.len() / src_channels as usize;
                let output_samples = (input_frames as f64 * ratio) as usize;

                if let Ok(mut guard) = producer.lock() {
                    for i in 0..output_samples {
                        let src_pos = i as f64 / ratio;
                        let idx = src_pos.floor() as usize;
                        let frac = (src_pos - idx as f64) as f32;

                        if idx + 1 < input_frames {
                            let mut mono = 0.0f32;
                            for ch in 0..src_channels as usize {
                                let a = data[idx * src_channels as usize + ch];
                                let b = data[(idx + 1) * src_channels as usize + ch];
                                mono += a * (1.0 - frac) + b * frac;
                            }
                            mono /= src_channels as f32;
                            let _ = guard.try_push(mono);
                        }
                    }
                }
            },
            |err| {
                error!("audio error: {}", err);
            },
            None,
        )
        .map_err(|e| format!("stream error: {}", e))?;

    stream.play().map_err(|e| format!("play error: {}", e))?;
    Ok(stream)
}

fn build_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    src_rate: u32,
    src_channels: u16,
    target_rate: u32,
    producer: Arc<Mutex<ringbuf::HeapProd<f32>>>,
) -> Result<Stream, String> {
    let ratio = target_rate as f64 / src_rate as f64;

    let stream = device
        .build_input_stream(
            config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let input_frames = data.len() / src_channels as usize;
                let output_samples = (input_frames as f64 * ratio) as usize;

                if let Ok(mut guard) = producer.lock() {
                    for i in 0..output_samples {
                        let src_pos = i as f64 / ratio;
                        let idx = src_pos.floor() as usize;
                        let frac = (src_pos - idx as f64) as f32;

                        if idx + 1 < input_frames {
                            let mut mono = 0.0f32;
                            for ch in 0..src_channels as usize {
                                let a = data[idx * src_channels as usize + ch] as f32 / 32768.0;
                                let b =
                                    data[(idx + 1) * src_channels as usize + ch] as f32 / 32768.0;
                                mono += a * (1.0 - frac) + b * frac;
                            }
                            mono /= src_channels as f32;
                            let _ = guard.try_push(mono);
                        }
                    }
                }
            },
            |err| {
                error!("audio error: {}", err);
            },
            None,
        )
        .map_err(|e| format!("stream error: {}", e))?;

    stream.play().map_err(|e| format!("play error: {}", e))?;
    Ok(stream)
}

fn build_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    src_rate: u32,
    src_channels: u16,
    target_rate: u32,
    producer: Arc<Mutex<ringbuf::HeapProd<f32>>>,
) -> Result<Stream, String> {
    let ratio = target_rate as f64 / src_rate as f64;

    let stream = device
        .build_input_stream(
            config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                let input_frames = data.len() / src_channels as usize;
                let output_samples = (input_frames as f64 * ratio) as usize;

                if let Ok(mut guard) = producer.lock() {
                    for i in 0..output_samples {
                        let src_pos = i as f64 / ratio;
                        let idx = src_pos.floor() as usize;
                        let frac = (src_pos - idx as f64) as f32;

                        if idx + 1 < input_frames {
                            let mut mono = 0.0f32;
                            for ch in 0..src_channels as usize {
                                let a = (data[idx * src_channels as usize + ch] as f32 - 32768.0)
                                    / 32768.0;
                                let b = (data[(idx + 1) * src_channels as usize + ch] as f32
                                    - 32768.0)
                                    / 32768.0;
                                mono += a * (1.0 - frac) + b * frac;
                            }
                            mono /= src_channels as f32;
                            let _ = guard.try_push(mono);
                        }
                    }
                }
            },
            |err| {
                error!("audio error: {}", err);
            },
            None,
        )
        .map_err(|e| format!("stream error: {}", e))?;

    stream.play().map_err(|e| format!("play error: {}", e))?;
    Ok(stream)
}
