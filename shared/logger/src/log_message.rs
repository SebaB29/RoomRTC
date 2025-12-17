//! Internal log message structure.

use crate::log_level::LogLevel;
use chrono::Local;

/// Internal representation of a log message.
#[derive(Debug, Clone)]
pub(crate) struct LogMessage {
    pub timestamp: String,
    pub level: LogLevel,
    pub component: Option<String>,
    pub message: String,
}

impl LogMessage {
    /// Creates a new log message with current timestamp.
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            level,
            component: None,
            message,
        }
    }

    /// Creates a new log message with component/layer information.
    pub fn new_with_component(level: LogLevel, component: String, message: String) -> Self {
        Self {
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            level,
            component: Some(component),
            message,
        }
    }

    /// Formats message for file output: `[timestamp] LEVEL [component]: message\n`
    pub fn format(&self) -> String {
        if let Some(ref component) = self.component {
            format!(
                "[{}] {} [component: {}]: {}\n",
                self.timestamp,
                self.level.as_str(),
                component,
                self.message
            )
        } else {
            format!(
                "[{}] {}: {}\n",
                self.timestamp,
                self.level.as_str(),
                self.message
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_message_creation() {
        let msg = LogMessage::new(LogLevel::Info, "Test message".to_string());

        assert_eq!(msg.level, LogLevel::Info);
        assert_eq!(msg.message, "Test message");
        assert!(!msg.timestamp.is_empty());
    }

    #[test]
    fn test_log_message_format() {
        let msg = LogMessage::new(LogLevel::Error, "Connection failed".to_string());
        let formatted = msg.format();

        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("Connection failed"));
        assert!(formatted.ends_with('\n'));
    }

    #[test]
    fn test_timestamp_format() {
        let msg = LogMessage::new(LogLevel::Info, "Test".to_string());
        let ts = &msg.timestamp;

        // Should match YYYY-MM-DD HH:MM:SS.mmm format
        assert!(ts.len() >= 23);
        assert!(ts.contains('-'));
        assert!(ts.contains(':'));
        assert!(ts.contains('.'));
    }
}
