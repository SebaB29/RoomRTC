/// Audio capture using cpal
use crate::error::{MediaError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use logging::Logger;
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    buffer: Arc<Mutex<Vec<i16>>>,
    _stream: Option<cpal::Stream>,
    sample_rate: u32,
    channels: u32,
}

// SAFETY: The stream is never accessed directly after creation.
// All audio processing happens in the stream's callback, which is managed by cpal.
// The buffer is thread-safe (Arc<Mutex<>>).
unsafe impl Send for AudioCapture {}

/// Limits the audio buffer to approximately 2 seconds
/// Removes oldest samples if the buffer exceeds the limit
#[inline]
fn limit_buffer_size(buf: &mut Vec<i16>, channels: usize) {
    const MAX_SECONDS: usize = 2;
    const SAMPLE_RATE: usize = 48000;
    let max_samples = SAMPLE_RATE * channels * MAX_SECONDS;
    let current_len = buf.len();
    if current_len > max_samples {
        buf.drain(0..(current_len - max_samples));
    }
}

impl AudioCapture {
    pub fn new(
        device_id: Option<i32>,
        sample_rate: u32,
        channels: u32,
        logger: &Logger,
    ) -> Result<Self> {
        logger.info(&format!(
            "Initializing cpal audio capture: {} Hz, {} channels, device: {:?}",
            sample_rate, channels, device_id
        ));

        let buffer = Arc::new(Mutex::new(Vec::new()));

        let host = cpal::default_host();
        let device = if let Some(_id) = device_id {
            // Por ahora usar default, pero se puede mejorar para seleccionar por ID
            host.default_input_device()
                .ok_or_else(|| MediaError::Audio("No input device available".into()))?
        } else {
            host.default_input_device()
                .ok_or_else(|| MediaError::Audio("No input device available".into()))?
        };

        logger.info(&format!(
            "Using input device: {}",
            device.name().unwrap_or_default()
        ));

        let config = device
            .default_input_config()
            .map_err(|e| MediaError::Audio(format!("Failed to get default input config: {}", e)))?;

        logger.info(&format!(
            "Input format: {:?}, {} Hz, {} channels",
            config.sample_format(),
            config.sample_rate().0,
            config.channels()
        ));

        let buffer_clone = Arc::clone(&buffer);
        let channels_usize = config.channels() as usize;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let config: cpal::StreamConfig = config.into();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let mut buf = buffer_clone.lock().expect("Audio buffer lock poisoned");
                        // Convertir f32 [-1.0, 1.0] a i16 con mejor precisión
                        for &sample in data {
                            // Clamp y redondear para evitar distorsión
                            let clamped = sample.clamp(-1.0, 1.0);
                            let scaled = clamped * 32767.0;
                            let sample_i16 = scaled.round() as i16;
                            buf.push(sample_i16);
                        }
                        limit_buffer_size(&mut buf, channels_usize);
                    },
                    |err| eprintln!("Audio input error: {}", err),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let config: cpal::StreamConfig = config.into();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let mut buf = buffer_clone.lock().expect("Audio buffer lock poisoned");
                        buf.extend_from_slice(data);
                        limit_buffer_size(&mut buf, channels_usize);
                    },
                    |err| eprintln!("Audio input error: {}", err),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let config: cpal::StreamConfig = config.into();
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let mut buf = buffer_clone.lock().expect("Audio buffer lock poisoned");
                        // Convertir u16 a i16
                        for &sample in data {
                            let sample_i16 = (sample as i32 - 32768) as i16;
                            buf.push(sample_i16);
                        }
                        limit_buffer_size(&mut buf, channels_usize);
                    },
                    |err| eprintln!("Audio input error: {}", err),
                    None,
                )
            }
            _ => return Err(MediaError::Audio("Unsupported input sample format".into())),
        }
        .map_err(|e| MediaError::Audio(format!("Failed to build input stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| MediaError::Audio(format!("Failed to start input stream: {}", e)))?;

        logger.info("Audio capture stream started");

        Ok(Self {
            buffer,
            _stream: Some(stream),
            sample_rate,
            channels,
        })
    }

    /// Captura samples del buffer
    pub fn read_samples(&self, requested_samples: usize) -> Vec<i16> {
        let mut buf = self.buffer.lock().expect("Audio buffer lock poisoned");

        if buf.len() >= requested_samples {
            // Tenemos suficientes samples
            let samples: Vec<i16> = buf.drain(0..requested_samples).collect();
            samples
        } else if !buf.is_empty() {
            // Devolver lo que hay y rellenar con ceros
            let available = buf.len();
            let mut samples = buf.drain(0..available).collect::<Vec<i16>>();
            samples.resize(requested_samples, 0);
            samples
        } else {
            // Buffer vacío, devolver silencio
            vec![0i16; requested_samples]
        }
    }

    /// Clears all buffered samples
    pub fn clear_buffer(&self) {
        let mut buf = self.buffer.lock().expect("Audio buffer lock poisoned");
        buf.clear();
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u32 {
        self.channels
    }
}
