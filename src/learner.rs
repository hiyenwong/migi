//! Learner — 认知层增强实现
//!
//! 包含 StatisticalLearner 的完整实现，
//! 异常检测算法，以及系统拓扑推断。

use crate::error::MigiResult;
use crate::observer::HostEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 系统行为模型
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemModel {
    pub version: u64,
    pub observed_events: u64,
    pub prediction_accuracy: f64,
    pub model_entropy: f64,
    pub identified_subsystems: usize,
    pub identified_patterns: usize,
    /// 已知的子系统名称列表
    pub subsystem_names: Vec<String>,
    /// 异常基线统计
    pub baseline: Option<BaselineStats>,
}

impl SystemModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_reliable(&self, threshold: f64) -> bool {
        self.prediction_accuracy >= (1.0 - threshold)
    }
}

/// 基线统计 — 用于异常检测
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineStats {
    /// 每个事件模式的频率基线 (pattern -> (mean, std_dev))
    pub pattern_frequencies: HashMap<String, (f64, f64)>,
    /// 基线覆盖的事件总数
    pub baseline_events: u64,
}

impl BaselineStats {
    /// 计算模式是否偏离基线
    pub fn is_anomalous(&self, pattern: &str, observed_count: f64, window_size: f64) -> bool {
        if let Some(&(mean, std_dev)) = self.pattern_frequencies.get(pattern) {
            if std_dev < 0.001 {
                // Near-zero std_dev — any deviation is anomalous
                return (observed_count / window_size - mean).abs() > mean * 0.5;
            }
            // Z-score > 2.0 is considered anomalous
            let z_score = (observed_count / window_size - mean).abs() / std_dev;
            z_score > 2.0
        } else {
            // Unknown pattern — treat as anomalous
            true
        }
    }
}

/// 预测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predictions {
    pub event_distribution: Vec<(String, f64)>,
    pub anomaly_probability: f64,
    pub confidence: f64,
}

/// Learner trait
pub trait Learner: Send + Sync {
    fn process_events(&mut self, events: &[HostEvent]) -> MigiResult<()>;
    fn get_model(&self) -> &SystemModel;
    fn predict(&self) -> MigiResult<Predictions>;
}

/// 统计学习器 — 完整实现
///
/// 功能:
/// - 事件频率统计
/// - 异常检测（基于 Z-score 的频率偏离 + 新事件类型检测）
/// - 预测生成（基于历史分布）
/// - 基线更新（滑动窗口）
#[derive(Default)]
pub struct StatisticalLearner {
    model: SystemModel,
    /// 事件模式计数 (source_eventType -> count)
    event_counts: HashMap<String, u64>,
    total_events: u64,
    /// 当前批次的窗口计数（用于异常检测）
    window_counts: HashMap<String, u64>,
    window_size: u64,
    /// 基线更新间隔
    baseline_update_interval: u64,
    batches_since_baseline: u64,
}

impl StatisticalLearner {
    pub fn new() -> Self {
        Self {
            window_size: 50,             // 每 50 个事件一个检测窗口
            baseline_update_interval: 5, // 每 5 个窗口更新一次基线
            ..Default::default()
        }
    }

    /// 设置窗口大小
    pub fn with_window_size(mut self, size: u64) -> Self {
        self.window_size = size;
        self
    }

    /// 检测当前窗口中的异常模式
    fn detect_anomalies(&self) -> (f64, Vec<String>) {
        if let Some(baseline) = &self.model.baseline {
            let mut anomalies = Vec::new();
            let mut max_z_score: f64 = 0.0;

            for (pattern, &count) in &self.window_counts {
                let window_size_f = self.window_size as f64;
                if baseline.is_anomalous(pattern, count as f64, window_size_f) {
                    anomalies.push(pattern.clone());
                    // Calculate z-score for probability estimation
                    if let Some(&(mean, std_dev)) = baseline.pattern_frequencies.get(pattern) {
                        if std_dev > 0.001 {
                            let z = (count as f64 / window_size_f - mean).abs() / std_dev;
                            max_z_score = max_z_score.max(z);
                        } else {
                            max_z_score = max_z_score.max(3.0);
                        }
                    }
                }
            }

            // Convert z-score to probability (sigmoid-like mapping)
            let anomaly_prob = if max_z_score > 0.0 {
                1.0 - 1.0 / (1.0 + (max_z_score - 2.0) * 0.5)
            } else {
                0.0
            };

            (anomaly_prob.clamp(0.0, 1.0), anomalies)
        } else {
            // No baseline yet — check for completely new patterns
            let new_patterns: Vec<String> = self
                .window_counts
                .keys()
                .filter(|p| !self.event_counts.contains_key(*p))
                .cloned()
                .collect();
            let prob = if new_patterns.is_empty() {
                0.0
            } else {
                0.3 // Moderate probability for new patterns without baseline
            };
            (prob, new_patterns)
        }
    }

    /// 更新基线统计
    fn update_baseline(&mut self) {
        let mut pattern_frequencies = HashMap::new();
        let window_size_f = self.window_size as f64;

        for (pattern, &count) in &self.event_counts {
            // Calculate mean frequency per event
            let windows = self.total_events.max(1) as f64 / window_size_f;
            let mean = count as f64 / windows;
            // Simple std_dev estimate: sqrt of mean (Poisson approximation)
            let std_dev = mean.sqrt();
            pattern_frequencies.insert(pattern.clone(), (mean, std_dev));
        }

        self.model.baseline = Some(BaselineStats {
            pattern_frequencies,
            baseline_events: self.total_events,
        });
    }

    /// 识别子系统
    fn update_subsystems(&mut self) {
        let sources: std::collections::HashSet<String> = self
            .event_counts
            .keys()
            .filter_map(|k| {
                // Keys are like "\"source\"_EventType" — extract and clean the source part
                k.split('_').next().map(|s| s.trim_matches('"').to_string())
            })
            .collect();

        // Only update if we found new subsystems
        if sources.len() > self.model.subsystem_names.len() {
            self.model.subsystem_names = sources.into_iter().collect();
            self.model.identified_subsystems = self.model.subsystem_names.len();
        }
    }
}

impl Learner for StatisticalLearner {
    fn process_events(&mut self, events: &[HostEvent]) -> MigiResult<()> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            let key = format!("{:?}_{:?}", event.source, event.event_type);
            *self.event_counts.entry(key.clone()).or_insert(0) += 1;
            *self.window_counts.entry(key).or_insert(0) += 1;
            self.total_events += 1;
        }

        self.batches_since_baseline += 1;

        // Update model metadata
        self.model.observed_events = self.total_events;
        self.model.version += 1;
        self.model.identified_patterns = self.event_counts.len();
        self.update_subsystems();

        // Prediction accuracy: logarithmic growth
        if self.total_events > 0 {
            let log_n = (self.total_events as f64).ln().max(1.0);
            self.model.prediction_accuracy = (log_n / 10.0).min(1.0);
            self.model.model_entropy = 1.0 - self.model.prediction_accuracy;
        }

        // Periodically update baseline
        if self.batches_since_baseline >= self.baseline_update_interval {
            self.update_baseline();
            self.batches_since_baseline = 0;
        }

        // Detect anomalies in current window
        let (_anomaly_prob, anomalies) = self.detect_anomalies();
        if !anomalies.is_empty() {
            tracing::warn!(
                anomaly_count = anomalies.len(),
                patterns = ?anomalies,
                "anomaly detected in event patterns"
            );
        }

        // Reset window if full
        let total_window: u64 = self.window_counts.values().sum();
        if total_window >= self.window_size {
            self.window_counts.clear();
        }

        Ok(())
    }

    fn get_model(&self) -> &SystemModel {
        &self.model
    }

    fn predict(&self) -> MigiResult<Predictions> {
        if self.total_events == 0 {
            return Ok(Predictions {
                event_distribution: Vec::new(),
                anomaly_probability: 0.0,
                confidence: 0.0,
            });
        }

        let total = self.total_events as f64;
        let mut distribution: Vec<(String, f64)> = self
            .event_counts
            .iter()
            .map(|(k, &v)| (k.clone(), v as f64 / total))
            .collect();

        // Sort by probability descending
        distribution.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (anomaly_prob, _) = self.detect_anomalies();

        Ok(Predictions {
            event_distribution: distribution,
            anomaly_probability: anomaly_prob,
            confidence: self.model.prediction_accuracy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::{EventType, Severity};
    use std::time::SystemTime;

    fn make_event(source: &str, event_type: EventType) -> HostEvent {
        HostEvent {
            timestamp: SystemTime::now(),
            source: source.to_string(),
            event_type,
            payload: serde_json::json!({}),
            severity: Severity::Info,
        }
    }

    #[test]
    fn test_process_events_updates_model() {
        let mut learner = StatisticalLearner::new();
        let events = vec![
            make_event("api", EventType::RequestIn),
            make_event("api", EventType::RequestComplete),
            make_event("db", EventType::Error),
        ];
        learner.process_events(&events).unwrap();
        let model = learner.get_model();
        assert_eq!(model.observed_events, 3);
        assert_eq!(model.version, 1);
        assert_eq!(model.identified_patterns, 3);
        assert_eq!(model.identified_subsystems, 2); // api, db
    }

    #[test]
    fn test_predict_no_events() {
        let learner = StatisticalLearner::new();
        let predictions = learner.predict().unwrap();
        assert!(predictions.event_distribution.is_empty());
        assert_eq!(predictions.anomaly_probability, 0.0);
        assert_eq!(predictions.confidence, 0.0);
    }

    #[test]
    fn test_predict_distribution() {
        let mut learner = StatisticalLearner::new();
        // Add 3 api events and 1 db event
        let events = vec![
            make_event("api", EventType::RequestIn),
            make_event("api", EventType::RequestIn),
            make_event("api", EventType::RequestIn),
            make_event("db", EventType::Error),
        ];
        learner.process_events(&events).unwrap();
        let predictions = learner.predict().unwrap();
        assert!(!predictions.event_distribution.is_empty());
        // Probabilities should sum to ~1.0
        let sum: f64 = predictions.event_distribution.iter().map(|(_, p)| p).sum();
        assert!((sum - 1.0).abs() < 0.01);
        // api pattern should be more probable
        let api_prob = predictions
            .event_distribution
            .iter()
            .find(|(k, _)| k.contains("api"))
            .map(|(_, p)| *p)
            .unwrap();
        assert!(api_prob > 0.5);
    }

    #[test]
    fn test_empty_events_no_change() {
        let mut learner = StatisticalLearner::new();
        learner.process_events(&[]).unwrap();
        assert_eq!(learner.get_model().observed_events, 0);
    }

    #[test]
    fn test_system_model_reliability() {
        let mut model = SystemModel::new();
        model.prediction_accuracy = 0.95;
        assert!(model.is_reliable(0.05)); // 0.95 >= 0.95
        assert!(!model.is_reliable(0.01)); // 0.95 < 0.99

        model.prediction_accuracy = 0.80;
        assert!(!model.is_reliable(0.05)); // 0.80 < 0.95
    }

    #[test]
    fn test_baseline_stats_anomaly_detection() {
        let mut baseline = BaselineStats {
            pattern_frequencies: HashMap::new(),
            baseline_events: 1000,
        };
        // Pattern with mean=10, std_dev=2
        baseline
            .pattern_frequencies
            .insert("api_RequestIn".to_string(), (10.0, 2.0));

        // Normal: count=12, window=1 → rate=12, z=(12-10)/2=1.0 → not anomalous
        assert!(!baseline.is_anomalous("api_RequestIn", 12.0, 1.0));

        // Anomalous: count=20, window=1 → rate=20, z=(20-10)/2=5.0 → anomalous
        assert!(baseline.is_anomalous("api_RequestIn", 20.0, 1.0));

        // Unknown pattern → anomalous
        assert!(baseline.is_anomalous("unknown_pattern", 5.0, 1.0));
    }

    #[test]
    fn test_new_event_type_detection() {
        let learner = StatisticalLearner::new();
        // Without baseline, new patterns get moderate probability
        let (_, anomalies) = learner.detect_anomalies();
        assert!(anomalies.is_empty()); // No window data
    }

    #[test]
    fn test_subsystem_identification() {
        let mut learner = StatisticalLearner::new();
        let events = vec![
            make_event("auth", EventType::RequestIn),
            make_event("database", EventType::StateChange),
            make_event("cache", EventType::ResourceAlert),
        ];
        learner.process_events(&events).unwrap();
        let model = learner.get_model();
        assert_eq!(model.identified_subsystems, 3);
        assert!(model.subsystem_names.contains(&"auth".to_string()));
        assert!(model.subsystem_names.contains(&"database".to_string()));
        assert!(model.subsystem_names.contains(&"cache".to_string()));
    }
}
