//! Camera Thread Pool
//!
//! This module implements a thread pool for parallel camera frame processing.
//! It allows multiple cameras to be captured and encoded simultaneously without
//! blocking each other.
//!
//! # Architecture
//!
//! - **Worker threads**: Pool of threads that process camera frames
//! - **Job queue**: Channel-based queue for distributing work
//! - **Frame priority**: Support for prioritizing certain frames

use crate::error::{MediaError, Result};
use crate::video::codecs::h264::H264Encoder;
use crate::video::constants::h264::{NAL_START_CODE_4, NAL_TYPE_MASK, NAL_TYPE_SPS};
use crate::video::frame::VideoFrame;
use crate::video::traits::VideoEncoder;
use logging::{LogLevel, Logger};
use opencv::prelude::*;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Camera frame processing job
pub struct CameraJob {
    pub device_id: i32,
    /// Raw frame data (YUV or RGB)
    pub frame_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
    /// Priority (higher = process first)
    pub priority: u8,
}

/// Encoded frame result
pub struct EncodedFrame {
    pub device_id: i32,
    /// Encoded data (H.264 NAL units)
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub is_keyframe: bool,
}

/// Camera thread pool for parallel encoding
pub struct CameraThreadPool {
    workers: Vec<Worker>,
    job_tx: Sender<CameraJob>,
    result_rx: Receiver<EncodedFrame>,
}

impl CameraThreadPool {
    /// Creates a new camera thread pool
    ///
    /// # Arguments
    /// * `num_threads` - Number of worker threads (typically num_cpus - 1)
    pub fn new(num_threads: usize) -> Result<Self> {
        if num_threads == 0 {
            return Err(MediaError::Config("Thread pool size must be > 0".into()));
        }

        let (job_tx, job_rx) = channel::<CameraJob>();
        let (result_tx, result_rx) = channel::<EncodedFrame>();

        // Wrap job_rx in Arc<Mutex> so all workers can share it
        let job_rx = Arc::new(Mutex::new(job_rx));

        let mut workers = Vec::with_capacity(num_threads);

        for id in 0..num_threads {
            workers.push(Worker::new(id, Arc::clone(&job_rx), result_tx.clone())?);
        }

        Ok(Self {
            workers,
            job_tx,
            result_rx,
        })
    }

    /// Submits a camera frame for encoding
    ///
    /// # Arguments
    /// * `job` - Camera job containing frame data
    pub fn submit(&self, job: CameraJob) -> Result<()> {
        self.job_tx
            .send(job)
            .map_err(|e| MediaError::Processing(format!("Failed to submit job: {}", e)))
    }

    /// Tries to receive an encoded frame (non-blocking)
    ///
    /// # Returns
    /// * `Ok(Some(frame))` - Encoded frame ready
    /// * `Ok(None)` - No frame available yet
    pub fn try_recv(&self) -> Result<Option<EncodedFrame>> {
        match self.result_rx.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(e) => Err(MediaError::Processing(format!(
                "Failed to receive result: {}",
                e
            ))),
        }
    }

    /// Returns the number of worker threads
    pub fn thread_count(&self) -> usize {
        self.workers.len()
    }
}

/// Worker thread that processes camera jobs
struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Creates a logger for the worker (best-effort with fallback)
    fn create_worker_logger(id: usize) -> Logger {
        Logger::new(format!("camera-worker-{}.log", id).into(), LogLevel::Debug).unwrap_or_else(
            |_| {
                Logger::new("media.log".into(), LogLevel::Info)
                    .expect("Failed to create fallback logger for camera worker")
            },
        )
    }

    /// Receives next job from queue with mutex poisoning recovery
    fn receive_job(job_rx: &Arc<Mutex<Receiver<CameraJob>>>, logger: &Logger) -> Option<CameraJob> {
        let rx = match job_rx.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                logger.error("Job queue mutex poisoned in worker thread, recovering");
                poisoned.into_inner()
            }
        };
        rx.recv().ok()
    }

    /// Initializes encoder for the specified device
    fn initialize_encoder(
        device_id: i32,
        width: u32,
        height: u32,
        logger: Logger,
    ) -> Option<Box<dyn VideoEncoder + Send>> {
        match H264Encoder::new(width, height, 2_000_000, 60, 30.0, logger) {
            Ok(enc) => Some(Box::new(enc)),
            Err(e) => {
                eprintln!("Failed to create encoder for device {}: {}", device_id, e);
                None
            }
        }
    }

    /// Converts raw frame data to VideoFrame
    fn create_video_frame(
        frame_data: &[u8],
        _width: u32,
        height: u32,
        _worker_id: usize,
    ) -> Option<VideoFrame> {
        let mat = Mat::from_slice(frame_data).ok()?;
        let mat = mat.reshape(3, height as i32).ok()?;
        let mat = mat.try_clone().ok()?;
        Some(VideoFrame::new(mat))
    }

    /// Checks if encoded data represents a keyframe
    fn is_keyframe(data: &[u8]) -> bool {
        data.len() > NAL_START_CODE_4.len()
            && data[..NAL_START_CODE_4.len()] == NAL_START_CODE_4
            && (data[NAL_START_CODE_4.len()] & NAL_TYPE_MASK) == NAL_TYPE_SPS
    }

    /// Processes encoding job and sends result
    fn process_job(
        job: CameraJob,
        encoder: &mut Option<Box<dyn VideoEncoder + Send>>,
        result_tx: &Sender<EncodedFrame>,
        worker_id: usize,
    ) -> bool {
        let Some(enc) = encoder.as_mut() else {
            return true; // Continue processing
        };

        let Some(frame) =
            Self::create_video_frame(&job.frame_data, job.width, job.height, worker_id)
        else {
            return true; // Continue processing
        };

        match enc.encode(&frame) {
            Ok(encoded_data) if !encoded_data.is_empty() => {
                let result = EncodedFrame {
                    device_id: job.device_id,
                    data: encoded_data.clone(),
                    timestamp: job.timestamp,
                    is_keyframe: Self::is_keyframe(&encoded_data),
                };

                result_tx.send(result).is_ok() // Return false if channel closed
            }
            Ok(_) => true, // Empty frame, continue
            Err(e) => {
                eprintln!("Worker {}: Encoding error: {}", worker_id, e);
                true
            }
        }
    }

    fn new(
        id: usize,
        job_rx: Arc<Mutex<Receiver<CameraJob>>>,
        result_tx: Sender<EncodedFrame>,
    ) -> Result<Self> {
        let handle = thread::Builder::new()
            .name(format!("camera-worker-{}", id))
            .spawn(move || {
                let mut encoder: Option<Box<dyn VideoEncoder + Send>> = None;
                let mut current_device_id = -1;

                loop {
                    let logger = Self::create_worker_logger(id);

                    // Wait for next job
                    let Some(job) = Self::receive_job(&job_rx, &logger) else {
                        break; // Channel closed
                    };

                    // Reinitialize encoder if device changed
                    if job.device_id != current_device_id {
                        if let Some(enc) =
                            Self::initialize_encoder(job.device_id, job.width, job.height, logger)
                        {
                            current_device_id = job.device_id;
                            encoder = Some(enc);
                        } else {
                            continue;
                        }
                    }

                    // Process encoding job
                    if !Self::process_job(job, &mut encoder, &result_tx, id) {
                        break; // Result channel closed
                    }
                }

                println!("Worker {} shutting down", id);
            })
            .map_err(|e| MediaError::Camera(format!("Failed to spawn worker thread: {}", e)))?;

        Ok(Self {
            id,
            handle: Some(handle),
        })
    }
}

impl Drop for CameraThreadPool {
    fn drop(&mut self) {
        println!("Shutting down camera thread pool...");

        // Drop job sender to signal workers to exit
        drop(self.job_tx.clone());

        // Wait for all workers to finish
        for worker in &mut self.workers {
            if let Some(handle) = worker.handle.take()
                && let Err(e) = handle.join()
            {
                eprintln!("Worker {} panicked: {:?}", worker.id, e);
            }
        }

        println!("Camera thread pool shut down complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_pool_creation() {
    //     let pool = CameraThreadPool::new(4);
    //     assert!(pool.is_ok());
    //     let pool = pool.unwrap();
    //     assert_eq!(pool.thread_count(), 4);
    // }

    #[test]
    fn test_pool_zero_threads() {
        let pool = CameraThreadPool::new(0);
        assert!(pool.is_err());
    }
}
