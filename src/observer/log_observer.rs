//! LogObserver — 日志流观察者
//!
//! 通过 tail 日志文件观察宿主系统行为。

use crate::error::MigiResult;
use crate::observer::{EventType, HostEvent, ObservationChannel, Severity};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};

/// 日志文件观察者
pub struct LogObserver {
    file_path: String,
    file: Option<BufReader<File>>,
    running: bool,
}

impl LogObserver {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            file: None,
            running: false,
        }
    }

    /// 解析日志行为 HostEvent
    ///
    /// 预期格式: `TIMESTAMP LEVEL MODULE MESSAGE`
    /// 例如: `2024-01-15T10:30:00Z ERROR auth Failed to authenticate user`
    pub fn parse_log_line(line: &str) -> Option<HostEvent> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Split into at most 4 parts: timestamp, level, module, message
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() < 3 {
            // If only one part, treat entire line as message with Info severity
            return Some(HostEvent {
                timestamp: SystemTime::now(),
                source: "unknown".to_string(),
                event_type: EventType::Custom("log_line".to_string()),
                payload: serde_json::json!({"message": line}),
                severity: Severity::Info,
            });
        }

        let timestamp = parse_timestamp(parts[0]).unwrap_or_else(SystemTime::now);
        let severity = parse_severity(parts[1]);
        let source = parts[2].to_string();
        let message = if parts.len() >= 4 {
            parts[3].to_string()
        } else {
            String::new()
        };

        let event_type = match severity {
            Severity::Error | Severity::Critical => EventType::Error,
            Severity::Warning => EventType::ResourceAlert,
            _ => {
                if message.contains("state change") || message.contains("status") {
                    EventType::StateChange
                } else {
                    EventType::RequestIn
                }
            }
        };

        Some(HostEvent {
            timestamp,
            source,
            event_type,
            payload: serde_json::json!({"message": message}),
            severity,
        })
    }
}

#[async_trait]
impl ObservationChannel for LogObserver {
    fn name(&self) -> &str {
        "LogObserver"
    }

    async fn start(&mut self) -> MigiResult<()> {
        let file = File::open(&self.file_path).await.map_err(|e| {
            crate::error::MigiError::Observer(format!(
                "failed to open log file '{}': {}",
                self.file_path, e
            ))
        })?;

        // Seek to end — tail mode
        let mut reader = BufReader::new(file);
        reader
            .seek(SeekFrom::End(0))
            .await
            .map_err(|e| crate::error::MigiError::Observer(format!("seek failed: {e}")))?;

        self.file = Some(reader);
        self.running = true;
        tracing::info!(file = %self.file_path, "LogObserver started");
        Ok(())
    }

    async fn next_event(&mut self) -> MigiResult<Option<HostEvent>> {
        if !self.running {
            return Ok(None);
        }

        let Some(reader) = &mut self.file else {
            return Ok(None);
        };

        let mut line = String::new();
        let bytes_read = reader
            .read_line(&mut line)
            .await
            .map_err(|e| crate::error::MigiError::Observer(format!("read failed: {e}")))?;

        if bytes_read == 0 {
            return Ok(None);
        }

        Ok(Self::parse_log_line(&line))
    }

    async fn stop(&mut self) -> MigiResult<()> {
        self.running = false;
        self.file = None;
        tracing::info!("LogObserver stopped");
        Ok(())
    }
}

/// 解析时间戳字符串
fn parse_timestamp(ts: &str) -> Option<std::time::SystemTime> {
    // Try ISO 8601 formats
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        return Some(dt.with_timezone(&Utc).into());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc().into());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc().into());
    }
    // Unix timestamp
    if let Ok(unix) = ts.parse::<i64>() {
        return Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(unix as u64));
    }
    None
}

/// 将日志级别映射到 Severity
fn parse_severity(level: &str) -> Severity {
    match level.to_uppercase().as_str() {
        "DEBUG" | "TRACE" => Severity::Debug,
        "INFO" => Severity::Info,
        "WARN" | "WARNING" => Severity::Warning,
        "ERROR" | "ERR" | "FATAL" => Severity::Error,
        "CRITICAL" | "CRIT" => Severity::Critical,
        _ => Severity::Info, // Unknown levels default to Info
    }
}

// Re-export SystemTime for parse_log_line
use std::time::SystemTime;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_severity_debug() {
        assert_eq!(parse_severity("DEBUG"), Severity::Debug);
        assert_eq!(parse_severity("TRACE"), Severity::Debug);
    }

    #[test]
    fn test_parse_severity_info() {
        assert_eq!(parse_severity("INFO"), Severity::Info);
        assert_eq!(parse_severity("UNKNOWN"), Severity::Info);
    }

    #[test]
    fn test_parse_severity_warning() {
        assert_eq!(parse_severity("WARN"), Severity::Warning);
        assert_eq!(parse_severity("WARNING"), Severity::Warning);
    }

    #[test]
    fn test_parse_severity_error() {
        assert_eq!(parse_severity("ERROR"), Severity::Error);
        assert_eq!(parse_severity("ERR"), Severity::Error);
        assert_eq!(parse_severity("FATAL"), Severity::Error);
    }

    #[test]
    fn test_parse_severity_critical() {
        assert_eq!(parse_severity("CRITICAL"), Severity::Critical);
        assert_eq!(parse_severity("CRIT"), Severity::Critical);
    }

    #[test]
    fn test_parse_severity_case_insensitive() {
        assert_eq!(parse_severity("error"), Severity::Error);
        assert_eq!(parse_severity("Error"), Severity::Error);
        assert_eq!(parse_severity("warn"), Severity::Warning);
    }

    #[test]
    fn test_parse_log_line_full() {
        let line = "2024-01-15T10:30:00Z ERROR auth Failed to authenticate user";
        let event = LogObserver::parse_log_line(line).expect("should parse");
        assert_eq!(event.source, "auth");
        assert_eq!(event.severity, Severity::Error);
        assert_eq!(event.event_type, EventType::Error);
    }

    #[test]
    fn test_parse_log_line_info() {
        let line = "2024-01-15T10:30:00Z INFO api Request received";
        let event = LogObserver::parse_log_line(line).expect("should parse");
        assert_eq!(event.source, "api");
        assert_eq!(event.severity, Severity::Info);
    }

    #[test]
    fn test_parse_log_line_warning() {
        let line = "2024-01-15T10:30:00Z WARN db Connection pool low";
        let event = LogObserver::parse_log_line(line).expect("should parse");
        assert_eq!(event.severity, Severity::Warning);
        assert_eq!(event.event_type, EventType::ResourceAlert);
    }

    #[test]
    fn test_parse_log_line_empty() {
        assert!(LogObserver::parse_log_line("").is_none());
        assert!(LogObserver::parse_log_line("   ").is_none());
    }

    #[test]
    fn test_parse_log_line_short() {
        // Only 2 parts — falls back to single-message event
        let event = LogObserver::parse_log_line("hello world").expect("should parse as fallback");
        assert_eq!(event.severity, Severity::Info);
    }

    #[test]
    fn test_parse_timestamp_iso() {
        let ts = parse_timestamp("2024-01-15T10:30:00Z");
        assert!(ts.is_some());
    }

    #[test]
    fn test_parse_timestamp_unix() {
        let ts = parse_timestamp("1705312200");
        assert!(ts.is_some());
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        let ts = parse_timestamp("not-a-timestamp");
        assert!(ts.is_none());
    }

    #[test]
    fn test_log_observer_name() {
        let obs = LogObserver::new("/tmp/test.log");
        assert_eq!(obs.name(), "LogObserver");
    }
}
