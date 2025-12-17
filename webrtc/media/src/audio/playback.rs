/// Simple cpal-based audio playback handler
use crate::error::{MediaError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use logging::Logger;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct AudioPlayback {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    _stream: Option<cpal::Stream>,
    prebuffer_size: usize,
}

// SAFETY: The stream is never accessed directly after creation.
// All audio processing happens in the stream's callback, which is managed by cpal.
// The buffer is thread-safe (Arc<Mutex<>>).
unsafe impl Send for AudioPlayback {}

impl AudioPlayback {
    pub fn new(source_sample_rate: u32, channels: u32, logger: &Logger) -> Result<Self> {
        logger.info(&format!(
            "Initializing cpal audio playback: {}Hz, {} channels",
            source_sample_rate, channels
        ));

        // Pre-buffer 200ms worth of audio to prevent underruns
        let prebuffer_size = (source_sample_rate as f32 * 0.20 * channels as f32) as usize;
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(prebuffer_size * 10)));
        let is_prebuffering = Arc::new(Mutex::new(true));

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| MediaError::Audio("No output device available".into()))?;

        logger.info(&format!(
            "Using output device: {}",
            device.name().unwrap_or_default()
        ));

        let default_config = device
            .default_output_config()
            .map_err(|e| MediaError::Audio(format!("Failed to get default config: {}", e)))?;

        let sample_rate = default_config.sample_rate().0;
        let device_channels = default_config.channels();

        logger.info(&format!(
            "Device config: {}Hz, {} channels",
            sample_rate, device_channels
        ));

        // We use the default config for the device to ensure compatibility
        // The callback will handle resampling if source_rate != device_rate
        let config: cpal::StreamConfig = default_config.into();

        // Calculate resampling ratio
        let resampling_ratio = source_sample_rate as f32 / sample_rate as f32;

        logger.info(&format!(
            "Resampling ratio: {:.4} (Source: {}, Device: {})",
            resampling_ratio, source_sample_rate, sample_rate
        ));

        // Warning if channel count mismatch (we don't handle channel mixing elegantly yet, just truncation/silence)
        if channels as u16 != device_channels {
            logger.warn(&format!(
                "Channel mismatch: Source {}, Device {}. Audio may be incorrect.",
                channels, device_channels
            ));
        }

        let buffer_clone = Arc::clone(&buffer);
        let is_prebuffering_clone = Arc::clone(&is_prebuffering);
        let mut underrun_count = 0usize;

        let mut fractional_pos = 0.0f32;

        let channel_count = channels as usize;
        let mut previous_frame = vec![0.0f32; channel_count];
        let mut next_frame = vec![0.0f32; channel_count];
        let mut has_next_frame = false;

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut buf = buffer_clone.lock().expect("Audio buffer lock poisoned");
                    let mut prebuffering = is_prebuffering_clone
                        .lock()
                        .expect("Prebuffering lock poisoned");
                    let available = buf.len();

                    // We need enough data to produce 'needed' output samples
                    // needed_input ~= data.len() * ratio
                    // But checking loose availability is safer

                    // Wait until prebuffer is filled before starting playback
                    if *prebuffering {
                        if available >= prebuffer_size {
                            *prebuffering = false;
                        } else {
                            // Still prebuffering, output silence
                            data.fill(0.0);
                            return;
                        }
                    }

                    // Process output samples
                    let output_channels = config.channels as usize;
                    // We assume source and output have same channel count for now or we just map 1:1 up to min
                    let process_channels = std::cmp::min(channel_count, output_channels);

                    let mut output_idx = 0;
                    while output_idx < data.len() {
                        // Ensure we have current and next frame for interpolation
                        if !has_next_frame {
                            if buf.len() >= channel_count {
                                // Shift next to previous
                                previous_frame.copy_from_slice(&next_frame);

                                // Pop new next frame
                                for sample in next_frame.iter_mut().take(channel_count) {
                                    *sample = buf.pop_front().unwrap_or(0.0);
                                }
                                has_next_frame = true;
                            } else {
                                // Underrun
                                underrun_count += 1;
                                if underrun_count > 10 {
                                    // allow some small underruns before reset
                                    *prebuffering = true;
                                    underrun_count = 0;
                                }

                                // Fill remainder with silence/fade
                                while output_idx < data.len() {
                                    data[output_idx] = 0.0;
                                    output_idx += 1;
                                }
                                return;
                            }
                        }

                        // Map channels
                        // data structure: [L, R, L, R...]
                        for c in 0..output_channels {
                            if output_idx >= data.len() {
                                break;
                            }

                            let sample_val = if c < process_channels {
                                // Linear interpolation
                                let prev = previous_frame[c];
                                let next = next_frame[c];
                                prev + (next - prev) * fractional_pos
                            } else {
                                0.0 // silence for extra channels
                            };

                            data[output_idx] = sample_val;
                            output_idx += 1;
                        }

                        // Advance position
                        fractional_pos += resampling_ratio;
                        while fractional_pos >= 1.0 {
                            fractional_pos -= 1.0;
                            // Need new frame
                            has_next_frame = false; // logic loop will fetch next

                            // Optimization: if we skipped multiple frames (downsampling), we need to consume more
                            // But usually ratio is close to 1.0 (48k <-> 44.1k) so loop handles it naturaly
                            // by setting has_next_frame=false and re-entering the fetch block above
                            if fractional_pos >= 1.0 {
                                // We are stepping very fast (output rate << source rate), consume more
                                if buf.len() >= channel_count {
                                    previous_frame.copy_from_slice(&next_frame);
                                    for sample in next_frame.iter_mut().take(channel_count) {
                                        *sample = buf.pop_front().unwrap_or(0.0);
                                    }
                                    fractional_pos -= 1.0;
                                    // Loop continues to check if still >= 1.0 or fetches again
                                } else {
                                    // Buffer empty during skip
                                    break;
                                }
                            }
                        }
                    }

                    underrun_count = 0;
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )
            .map_err(|e| MediaError::Audio(format!("Failed to build stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| MediaError::Audio(format!("Failed to play stream: {}", e)))?;

        logger.info("Audio playback stream started");

        Ok(Self {
            buffer,
            _stream: Some(stream),
            prebuffer_size,
        })
    }

    pub fn play_samples(&self, samples: &[i16]) {
        let samples_f32: Vec<f32> = samples
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect();

        let mut buf = self.buffer.lock().expect("Audio buffer lock poisoned");

        // Agregar al final del buffer
        buf.extend(samples_f32);

        // Limit buffer to ~1.5 seconds (adjusted from 3s) to prevent massive latency accumulation
        // but allow enough for network jitter
        let max_samples = self.prebuffer_size * 10;
        if buf.len() > max_samples {
            // Remove oldest samples
            let excess = buf.len() - max_samples;
            buf.drain(0..excess);
        }
    }
}
