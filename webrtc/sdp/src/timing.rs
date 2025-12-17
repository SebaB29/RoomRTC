//! SDP timing information.
//!
//! Defines when an SDP session starts and stops, or marks it as permanent.

/// Represents timing information (t=) in an SDP message as defined in RFC 4566.
///
/// The "t=" field specifies the start and stop times for the session.
/// If both values are zero, the session is considered permanent.
#[derive(Debug, Clone)]
pub struct Timing {
    pub start_time: u64,
    pub stop_time: u64,
}

impl Timing {
    /// Validates the timing field according to RFC 4566.
    ///
    /// The stop time must be **greater than or equal** to the start time,
    /// unless both values are `0`, which represents an unbounded (permanent) session.
    ///
    /// # Returns
    /// * `Ok(())` – If the timing values are valid.
    /// * `Err(SdpError::InvalidTiming)` – If the stop time is less than the start time.
    pub fn validate(&self) -> Result<(), crate::errors::SdpError> {
        // Stop time must be greater than or equal to start time unless both are 0
        if self.start_time != 0 && self.stop_time != 0 && self.stop_time < self.start_time {
            return Err(crate::errors::SdpError::InvalidTiming);
        }

        Ok(())
    }

    /// Parses a `t=` line from an SDP message into a [`Timing`] structure.
    ///
    /// The value must contain **exactly two space-separated numbers**:
    /// - The start time
    /// - The stop time
    ///
    /// # Arguments
    /// * `value` – The value string from the SDP line (without the `t=` prefix).
    ///
    /// # Returns
    /// * `Ok(Timing)` – If both numbers are valid and the format is correct.
    /// * `Err(SdpError::InvalidTimingFormat)` – If the line does not contain two valid integers.
    pub fn parse(value: &str) -> Result<Self, crate::errors::SdpError> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(crate::errors::SdpError::InvalidTimingFormat);
        }

        Ok(Timing {
            start_time: parts[0]
                .parse()
                .map_err(|_| crate::errors::SdpError::InvalidTimingFormat)?,
            stop_time: parts[1]
                .parse()
                .map_err(|_| crate::errors::SdpError::InvalidTimingFormat)?,
        })
    }
}

/// Provides a default implementation for [`Timing`].
impl Default for Timing {
    fn default() -> Self {
        Self {
            start_time: 0,
            stop_time: 0,
        }
    }
}

/// Implements the `Display` trait for [`Timing`], allowing it to be
/// formatted in standard SDP syntax.
///
/// The output is written as:
/// ```text
/// t=<start-time> <stop-time>\r\n
/// ```
impl std::fmt::Display for Timing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "t={} {}", self.start_time, self.stop_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_parse_valid() {
        let timing = Timing::parse("0 0").unwrap();
        assert_eq!(timing.start_time, 0);
        assert_eq!(timing.stop_time, 0);
    }

    #[test]
    fn test_timing_parse_invalid_format() {
        assert!(Timing::parse("0").is_err());
        assert!(Timing::parse("0 0 0").is_err());
        assert!(Timing::parse("").is_err());
    }

    #[test]
    fn test_timing_parse_invalid_numbers() {
        assert!(Timing::parse("abc def").is_err());
        assert!(Timing::parse("123 xyz").is_err());
    }

    #[test]
    fn test_timing_validate_permanent_session() {
        let timing = Timing {
            start_time: 0,
            stop_time: 0,
        };
        assert!(timing.validate().is_ok());
    }

    #[test]
    fn test_timing_validate_valid_range() {
        let timing = Timing {
            start_time: 100,
            stop_time: 200,
        };
        assert!(timing.validate().is_ok());
    }

    #[test]
    fn test_timing_validate_equal_times() {
        let timing = Timing {
            start_time: 100,
            stop_time: 100,
        };
        assert!(timing.validate().is_ok());
    }

    #[test]
    fn test_timing_validate_invalid_range() {
        let timing = Timing {
            start_time: 200,
            stop_time: 100,
        };
        assert!(timing.validate().is_err());
    }

    #[test]
    fn test_timing_default() {
        let timing = Timing::default();
        assert_eq!(timing.start_time, 0);
        assert_eq!(timing.stop_time, 0);
    }

    #[test]
    fn test_timing_display() {
        let timing = Timing {
            start_time: 3724394400,
            stop_time: 3724398000,
        };
        let display = format!("{}", timing);
        assert_eq!(display, "t=3724394400 3724398000\n");
    }
}
