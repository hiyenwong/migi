//! MetricsObserver — 指标轮询观察者
//!
//! 定期轮询宿主的指标端点（如 /metrics），检测资源使用异常。

use crate::error::{MigiError, MigiResult};
use crate::observer::{EventType, HostEvent, ObservationChannel, Severity};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 指标数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsData {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 指标轮询观察者
pub struct MetricsObserver {
    endpoint: String,
    interval: Duration,
    threshold: f64,
    running: bool,
    client: Option<reqwest::Client>,
}

impl MetricsObserver {
    pub fn new(endpoint: &str, interval_secs: u64, threshold: f64) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            interval: Duration::from_secs(interval_secs),
            threshold,
            running: false,
            client: None,
        }
    }

    /// 解析指标响应并生成 HostEvent
    pub fn parse_metrics_response(body: &str, threshold: f64) -> MigiResult<HostEvent> {
        let metrics: MetricsData = serde_json::from_str(body)
            .map_err(|e| MigiError::Observer(format!("failed to parse metrics JSON: {e}")))?;

        let exceeded = metrics.cpu_percent > threshold || metrics.memory_percent > threshold;

        let severity = if exceeded {
            if metrics.cpu_percent > threshold * 1.5 || metrics.memory_percent > threshold * 1.5 {
                Severity::Error
            } else {
                Severity::Warning
            }
        } else {
            Severity::Info
        };

        let event_type = if exceeded {
            EventType::ResourceAlert
        } else {
            EventType::StateChange
        };

        let payload = serde_json::json!({
            "cpu_percent": metrics.cpu_percent,
            "memory_percent": metrics.memory_percent,
            "threshold": threshold,
            "exceeded": exceeded,
            "extra": metrics.extra,
        });

        Ok(HostEvent {
            timestamp: std::time::SystemTime::now(),
            source: "metrics".to_string(),
            event_type,
            payload,
            severity,
        })
    }
}

#[async_trait]
impl ObservationChannel for MetricsObserver {
    fn name(&self) -> &str {
        "MetricsObserver"
    }

    async fn start(&mut self) -> MigiResult<()> {
        self.client = Some(
            reqwest::Client::builder()
                .timeout(self.interval)
                .build()
                .map_err(|e| MigiError::Observer(format!("failed to build HTTP client: {e}")))?,
        );
        self.running = true;
        tracing::info!(
            endpoint = %self.endpoint,
            interval_secs = self.interval.as_secs(),
            threshold = self.threshold,
            "MetricsObserver started"
        );
        Ok(())
    }

    async fn next_event(&mut self) -> MigiResult<Option<HostEvent>> {
        if !self.running {
            return Ok(None);
        }

        let Some(client) = &self.client else {
            return Ok(None);
        };

        let response = client.get(&self.endpoint).send().await;

        match response {
            Ok(resp) => {
                let body = resp.text().await.map_err(|e| {
                    MigiError::Observer(format!("failed to read response body: {e}"))
                })?;
                let event = Self::parse_metrics_response(&body, self.threshold)?;
                Ok(Some(event))
            }
            Err(e) => {
                tracing::warn!(endpoint = %self.endpoint, error = %e, "metrics poll failed");
                // Return an error event instead of propagating
                Ok(Some(HostEvent {
                    timestamp: std::time::SystemTime::now(),
                    source: "metrics".to_string(),
                    event_type: EventType::Error,
                    payload: serde_json::json!({"error": e.to_string(), "endpoint": self.endpoint}),
                    severity: Severity::Warning,
                }))
            }
        }
    }

    async fn stop(&mut self) -> MigiResult<()> {
        self.running = false;
        self.client = None;
        tracing::info!("MetricsObserver stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metrics_response_normal() {
        let body = r#"{"cpu_percent": 45.0, "memory_percent": 60.0}"#;
        let event = MetricsObserver::parse_metrics_response(body, 80.0).unwrap();
        assert_eq!(event.severity, Severity::Info);
        assert_eq!(event.event_type, EventType::StateChange);
        assert_eq!(event.source, "metrics");
    }

    #[test]
    fn test_parse_metrics_response_warning() {
        let body = r#"{"cpu_percent": 85.0, "memory_percent": 60.0}"#;
        let event = MetricsObserver::parse_metrics_response(body, 80.0).unwrap();
        assert_eq!(event.severity, Severity::Warning);
        assert_eq!(event.event_type, EventType::ResourceAlert);
    }

    #[test]
    fn test_parse_metrics_response_error() {
        // 1.5x threshold → Error severity
        let body = r#"{"cpu_percent": 95.0, "memory_percent": 60.0}"#;
        let event = MetricsObserver::parse_metrics_response(body, 60.0).unwrap();
        assert_eq!(event.severity, Severity::Error);
        assert_eq!(event.event_type, EventType::ResourceAlert);
    }

    #[test]
    fn test_parse_metrics_response_memory_exceeded() {
        let body = r#"{"cpu_percent": 30.0, "memory_percent": 90.0}"#;
        let event = MetricsObserver::parse_metrics_response(body, 80.0).unwrap();
        assert_eq!(event.severity, Severity::Warning);
        assert_eq!(event.event_type, EventType::ResourceAlert);
    }

    #[test]
    fn test_parse_metrics_response_with_extra() {
        let body = r#"{"cpu_percent": 50.0, "memory_percent": 40.0, "disk_io": 120}"#;
        let event = MetricsObserver::parse_metrics_response(body, 80.0).unwrap();
        assert_eq!(event.severity, Severity::Info);
        assert!(event.payload.get("extra").is_some());
    }

    #[test]
    fn test_parse_metrics_response_invalid_json() {
        let result = MetricsObserver::parse_metrics_response("not json", 80.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_metrics_response_missing_fields() {
        let body = r#"{"foo": "bar"}"#;
        let result = MetricsObserver::parse_metrics_response(body, 80.0);
        assert!(result.is_err()); // serde defaults to 0.0, so this may succeed
                                  // Actually serde defaults f64 to 0.0 for missing fields, so let's verify:
        let body2 = r#"{"foo": "bar"}"#;
        let event = MetricsObserver::parse_metrics_response(body2, 80.0);
        // If it succeeds, both cpu_percent and memory_percent should be 0.0
        if let Ok(e) = event {
            assert_eq!(e.severity, Severity::Info);
        }
    }

    #[test]
    fn test_metrics_observer_name() {
        let obs = MetricsObserver::new("http://localhost:9090/metrics", 30, 80.0);
        assert_eq!(obs.name(), "MetricsObserver");
    }

    #[test]
    fn test_threshold_boundary() {
        // Exactly at threshold → not exceeded (uses > not >=)
        let body = r#"{"cpu_percent": 80.0, "memory_percent": 50.0}"#;
        let event = MetricsObserver::parse_metrics_response(body, 80.0).unwrap();
        assert_eq!(event.severity, Severity::Info);
        assert_eq!(event.event_type, EventType::StateChange);
    }
}
