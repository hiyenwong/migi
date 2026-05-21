//! Monitor — 监测 Migi 内部状态的工具
//!
//! Migi 运行时定期写入状态快照文件，migi-stats CLI 读取并展示。

use crate::config::SymbiosisPhase;
use crate::error::MigiResult;
use crate::learner::SystemModel;
use crate::trust::TrustState;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

/// 单次快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigiSnapshot {
    pub timestamp: u64, // unix秒
    pub phase: String,
    pub trust_score: f64,
    pub successful_interventions: u64,
    pub failed_interventions: u64,
    pub model_version: u64,
    pub observed_events: u64,
    pub model_accuracy: f64,
    pub model_entropy: f64,
    pub identified_patterns: usize,
    pub identified_subsystems: usize,
    /// 已发现子系统列表
    pub subsystems: Vec<String>,
    pub uptime_secs: u64,
}

impl MigiSnapshot {
    pub fn take(
        phase: &SymbiosisPhase,
        trust: &TrustState,
        model: &SystemModel,
        start_time: std::time::Instant,
    ) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            phase: format!("{:?}", phase),
            trust_score: trust.trust_score,
            successful_interventions: trust.successful_interventions,
            failed_interventions: trust.failed_interventions,
            model_version: model.version,
            observed_events: model.observed_events,
            model_accuracy: model.prediction_accuracy,
            model_entropy: model.model_entropy,
            identified_patterns: model.identified_patterns,
            identified_subsystems: model.identified_subsystems,
            subsystems: model.subsystem_names.clone(),
            uptime_secs: start_time.elapsed().as_secs(),
        }
    }

    /// 保存到文件
    pub fn save(&self, path: &Path) -> MigiResult<()> {
        let parent = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).ok();
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::MigiError::Observer(format!("snapshot serialization: {e}"))
        })?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &json)
            .map_err(|e| crate::error::MigiError::Observer(format!("snapshot write: {e}")))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| crate::error::MigiError::Observer(format!("snapshot persist: {e}")))?;
        Ok(())
    }

    /// 从文件加载
    pub fn load(path: &Path) -> MigiResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::MigiError::Observer(format!("read snapshot: {e}")))?;
        let snap: MigiSnapshot = serde_json::from_str(&content)
            .map_err(|e| crate::error::MigiError::Observer(format!("parse snapshot: {e}")))?;
        Ok(snap)
    }

    /// 格式化为人类可读字符串
    pub fn format(&self) -> String {
        let age_ago = std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp);
        let age_str = age_ago
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("{}s ago", d.as_secs()))
            .unwrap_or_else(|_| "unknown".into());

        format!(
            r#"╔══════════════════════════════════════╗
║         🦁 Migi Status            ║
╠══════════════════════════════════════╣
║  Phase:        {:<20} ║
║  Uptime:       {:>7}s              ║
║  Snapshot:     {:<20} ║
╠══════════════════════════════════════╣
║  📊 Model                            ║
║    Events:      {:>9}              ║
║    Accuracy:    {:.3}                 ║
║    Entropy:     {:.3}                 ║
║    Patterns:    {:>6}                ║
║    Subsystems:  {:>2} ({})   ║
║    Version:     {:>9}              ║
╠══════════════════════════════════════╣
║  🛡️ Trust                            ║
║    Score:       {:.3}                 ║
║    Successes:   {:>9}              ║
║    Failures:    {:>9}              ║
╚══════════════════════════════════════╝"#,
            self.phase,
            self.uptime_secs,
            age_str,
            self.observed_events,
            self.model_accuracy,
            self.model_entropy,
            self.identified_patterns,
            self.identified_subsystems,
            if self.subsystems.len() <= 5 {
                self.subsystems.join(", ")
            } else {
                format!("{}...", self.subsystems[..5].join(", "))
            },
            self.model_version,
            self.trust_score,
            self.successful_interventions,
            self.failed_interventions,
        )
    }

    /// 短格式 — 一行摘要
    pub fn summary(&self) -> String {
        format!(
            "[{:>7}s] {} | events={} | acc={:.3} | trust={:.3} | patterns={} | subsys={}",
            self.uptime_secs,
            self.phase,
            self.observed_events,
            self.model_accuracy,
            self.trust_score,
            self.identified_patterns,
            self.identified_subsystems,
        )
    }
}

/// 监测历史（保留最近 N 条快照用于趋势分析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorHistory {
    pub snapshots: Vec<MigiSnapshot>,
    pub capacity: usize,
}

impl MonitorHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            snapshots: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, snapshot: MigiSnapshot) {
        if self.snapshots.len() >= self.capacity {
            self.snapshots.remove(0);
        }
        self.snapshots.push(snapshot);
    }

    /// 保存历史到文件
    pub fn save(&self, path: &Path) -> MigiResult<()> {
        let parent = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).ok();
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::MigiError::Observer(format!("history serialization: {e}"))
        })?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &json)
            .map_err(|e| crate::error::MigiError::Observer(format!("history write: {e}")))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| crate::error::MigiError::Observer(format!("history persist: {e}")))?;
        Ok(())
    }

    /// 从文件加载
    pub fn load(path: &Path) -> MigiResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::MigiError::Observer(format!("read history: {e}")))?;
        let hist: MonitorHistory = serde_json::from_str(&content)
            .map_err(|e| crate::error::MigiError::Observer(format!("parse history: {e}")))?;
        Ok(hist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("migi_test_snapshot.json");
        let _ = std::fs::remove_file(&path);

        let model = SystemModel {
            version: 5,
            observed_events: 1500,
            prediction_accuracy: 0.85,
            model_entropy: 0.15,
            identified_patterns: 12,
            identified_subsystems: 3,
            subsystem_names: vec!["api".into(), "db".into(), "cache".into()],
            baseline: None,
        };
        let trust = crate::trust::TrustState::new(crate::config::SymbiosisPhase::Assistance);
        let snap = MigiSnapshot::take(
            &crate::config::SymbiosisPhase::Assistance,
            &trust,
            &model,
            std::time::Instant::now(),
        );
        snap.save(&path).unwrap();

        let loaded = MigiSnapshot::load(&path).unwrap();
        assert_eq!(loaded.phase, "Assistance");
        assert_eq!(loaded.observed_events, 1500);
        assert_eq!(loaded.identified_subsystems, 3);
        assert!(loaded.subsystems.contains(&"api".to_string()));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_history_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("migi_test_history.json");
        let _ = std::fs::remove_file(&path);

        let mut hist = MonitorHistory::new(3);
        let model = SystemModel::default();
        let trust = crate::trust::TrustState::new(crate::config::SymbiosisPhase::Observation);
        for _ in 0..5 {
            let snap = MigiSnapshot::take(
                &crate::config::SymbiosisPhase::Observation,
                &trust,
                &model,
                std::time::Instant::now(),
            );
            hist.push(snap);
        }
        hist.save(&path).unwrap();

        let loaded = MonitorHistory::load(&path).unwrap();
        assert_eq!(loaded.snapshots.len(), 3); // Capacity capped
        assert_eq!(loaded.capacity, 3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_summary_format() {
        let model = SystemModel::default();
        let trust = crate::trust::TrustState::new(crate::config::SymbiosisPhase::Observation);
        let snap = MigiSnapshot::take(
            &crate::config::SymbiosisPhase::Observation,
            &trust,
            &model,
            std::time::Instant::now(),
        );
        let summary = snap.summary();
        assert!(summary.contains("Observation"));
        assert!(summary.contains("events="));
        assert!(summary.contains("trust="));
    }
}
