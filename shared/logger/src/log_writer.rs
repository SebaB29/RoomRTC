//! Asynchronous log file writer.

use crate::error::Result;
use crate::log_message::LogMessage;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

/// Manages async log file writing in dedicated thread.
pub(crate) struct LogWriter {
    file: File,
}

impl LogWriter {
    /// Creates a new log writer, opening or creating the file in append mode.
    pub fn new(log_path: &PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        Ok(Self { file })
    }

    /// Writes and flushes a message to the file.
    fn write_message(&mut self, message: &LogMessage) {
        if let Err(e) = self.file.write_all(message.format().as_bytes()) {
            eprintln!("Error writing log: {}", e);
            return;
        }
        if let Err(e) = self.file.flush() {
            eprintln!("Error flushing log: {}", e);
        }
    }

    /// Runs the writer loop until channel closes.
    pub fn run(mut self, receiver: Receiver<LogMessage>) {
        for message in receiver {
            self.write_message(&message);
        }
    }
}

/// Spawns a dedicated log writer thread.
pub(crate) fn spawn_writer_thread(log_path: PathBuf, receiver: Receiver<LogMessage>) -> Result<()> {
    let writer = LogWriter::new(&log_path)?;
    std::thread::spawn(move || writer.run(receiver));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_level::LogLevel;
    use std::fs;
    use std::sync::mpsc::channel;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_log_writer_creation() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let writer = LogWriter::new(&log_path);
        assert!(writer.is_ok());
        assert!(log_path.exists());
    }

    #[test]
    fn test_write_message() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let mut writer = LogWriter::new(&log_path).unwrap();
        let message = LogMessage::new(LogLevel::Info, "Test message".to_string());

        writer.write_message(&message);

        let content = fs::read_to_string(log_path).unwrap();
        assert!(content.contains("INFO"));
        assert!(content.contains("Test message"));
    }

    #[test]
    fn test_spawn_writer_thread() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        let (sender, receiver) = channel();

        spawn_writer_thread(log_path.clone(), receiver).unwrap();

        sender
            .send(LogMessage::new(LogLevel::Debug, "Thread test".to_string()))
            .unwrap();
        drop(sender);

        thread::sleep(Duration::from_millis(100));

        let content = fs::read_to_string(log_path).unwrap();
        assert!(content.contains("Thread test"));
    }
}
