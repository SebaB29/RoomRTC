//! Thread-safe asynchronous logger implementation.
//!
//! This module provides the main [`Logger`] interface for logging messages
//! to a file without blocking the caller.

use crate::error::Result;
use crate::log_level::LogLevel;
use crate::log_message::LogMessage;
use crate::log_writer::spawn_writer_thread;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};

/// Thread-safe, non-blocking logger.
///
/// Cloneable instances share the same channel to a dedicated writer thread.
///
/// # Examples
///
/// ```
/// use logging::{Logger, LogLevel};
///
/// let logger = Logger::new("app.log".into(), LogLevel::Info).unwrap();
/// logger.info("Application started");
/// logger.error("Connection failed");
/// ```
#[derive(Clone)]
pub struct Logger {
    sender: Sender<LogMessage>,
    level: LogLevel,
    component: Option<String>,
    log_path: PathBuf,
    console_output: bool,
}

impl Logger {
    /// Creates a new logger with dedicated writer thread.
    ///
    /// # Arguments
    ///
    /// * `log_path` - Path to log file (created if it doesn't exist)
    /// * `level` - Minimum log level to record
    ///
    /// # Errors
    ///
    /// Returns error if the log file cannot be created or opened.
    pub fn new(log_path: PathBuf, level: LogLevel) -> Result<Self> {
        let (sender, receiver) = channel();
        spawn_writer_thread(log_path.clone(), receiver)?;
        Ok(Logger {
            sender,
            level,
            component: None,
            log_path,
            console_output: false,
        })
    }

    /// Creates a new logger with component/layer identification.
    ///
    /// # Arguments
    ///
    /// * `log_path` - Path to log file (created if it doesn't exist)
    /// * `level` - Minimum log level to record
    /// * `component` - Component or layer name (e.g., "HTTP", "TCP", "Storage")
    ///
    /// # Errors
    ///
    /// Returns error if the log file cannot be created or opened.
    pub fn with_component(
        log_path: PathBuf,
        level: LogLevel,
        component: String,
        console_output: bool,
    ) -> Result<Self> {
        let (sender, receiver) = channel();
        spawn_writer_thread(log_path.clone(), receiver)?;
        Ok(Logger {
            sender,
            level,
            component: Some(component),
            log_path,
            console_output,
        })
    }

    /// Creates a new logger with a different component but sharing the same configuration.
    ///
    /// # Arguments
    ///
    /// * `component` - New component or layer name
    ///
    /// # Errors
    ///
    /// Returns error if the log file cannot be created or opened.
    ///
    /// # Examples
    ///
    /// ```
    /// use logging::{Logger, LogLevel};
    ///
    /// let main_logger = Logger::new("app.log".into(), LogLevel::Info).unwrap();
    /// let db_logger = main_logger.for_component("Database").unwrap();
    /// ```
    pub fn for_component(&self, component: &str) -> Result<Self> {
        Self::with_component(
            self.log_path.clone(),
            self.level,
            component.to_string(),
            self.console_output,
        )
    }

    /// Logs a debug message (only if level is Debug or lower).
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    /// Logs an info message (only if level is Info or lower).
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// Logs a warning message (only if level is Warn or lower).
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    /// Logs an error message (always recorded).
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    /// Filters by level and sends message to writer thread.
    fn log(&self, level: LogLevel, message: &str) {
        if level >= self.level {
            let msg = if let Some(ref component) = self.component {
                LogMessage::new_with_component(level, component.clone(), message.to_string())
            } else {
                LogMessage::new(level, message.to_string())
            };

            // Send msg to console if enable
            if self.console_output {
                print!("{}", msg.clone().format())
            }

            let _ = self.sender.send(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    fn wait_for_write() {
        thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn test_logger_creates_file() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let logger = Logger::new(log_path.clone(), LogLevel::Debug).unwrap();
        logger.info("Test message");
        wait_for_write();

        assert!(log_path.exists());
        let content = fs::read_to_string(log_path).unwrap();
        assert!(content.contains("Test message"));
    }

    #[test]
    fn test_logger_respects_level() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let logger = Logger::new(log_path.clone(), LogLevel::Warn).unwrap();
        logger.debug("Debug message");
        logger.info("Info message");
        logger.warn("Warn message");
        wait_for_write();

        let content = fs::read_to_string(log_path).unwrap();
        assert!(!content.contains("Debug message"));
        assert!(!content.contains("Info message"));
        assert!(content.contains("Warn message"));
    }

    #[test]
    fn test_logger_clone_across_threads() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let logger = Logger::new(log_path.clone(), LogLevel::Info).unwrap();
        let logger_clone = logger.clone();

        thread::spawn(move || {
            logger_clone.info("Message from thread");
        });

        logger.info("Message from main");
        wait_for_write();

        let content = fs::read_to_string(log_path).unwrap();
        assert!(content.contains("Message from thread"));
        assert!(content.contains("Message from main"));
    }

    #[test]
    fn test_all_log_levels() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");

        let logger = Logger::new(log_path.clone(), LogLevel::Debug).unwrap();
        logger.debug("Debug message");
        logger.info("Info message");
        logger.warn("Warn message");
        logger.error("Error message");
        wait_for_write();

        let content = fs::read_to_string(log_path).unwrap();
        assert!(content.contains("DEBUG"));
        assert!(content.contains("INFO"));
        assert!(content.contains("WARN"));
        assert!(content.contains("ERROR"));
    }
}
