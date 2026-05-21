//! Trust — 信任层（增强版）
//!
//! 包含 TrustManager 的完整实现，
//! 状态持久化，以及相变门控逻辑。

use crate::config::SymbiosisPhase;
use crate::error::{MigiError, MigiResult};
use crate::intervener::{Action, Intervention};
use crate::learner::SystemModel;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 信任状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustState {
    pub phase: SymbiosisPhase,
    pub trust_score: f64,
    pub successful_interventions: u64,
    pub failed_interventions: u64,
    pub rejected_attempts: u64,
    pub consecutive_successes: u64,
}

impl TrustState {
    pub fn new(phase: SymbiosisPhase) -> Self {
        Self {
            phase,
            trust_score: 0.0,
            successful_interventions: 0,
            failed_interventions: 0,
            rejected_attempts: 0,
            consecutive_successes: 0,
        }
    }

    pub fn record_success(&mut self) {
        self.successful_interventions += 1;
        self.consecutive_successes += 1;
        self.update_score();
    }

    pub fn record_failure(&mut self) {
        self.failed_interventions += 1;
        self.consecutive_successes = 0;
        self.update_score();
    }

    pub fn record_rejection(&mut self) {
        self.rejected_attempts += 1;
    }

    fn update_score(&mut self) {
        let total = self.successful_interventions + self.failed_interventions;
        if total == 0 {
            self.trust_score = 0.0;
        } else {
            self.trust_score = self.successful_interventions as f64 / total as f64;
        }
    }
}

/// 可持久化的信任状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrustStateSnapshot {
    phase: SymbiosisPhase,
    trust_score: f64,
    successful_interventions: u64,
    failed_interventions: u64,
    rejected_attempts: u64,
    consecutive_successes: u64,
}

impl From<&TrustState> for TrustStateSnapshot {
    fn from(state: &TrustState) -> Self {
        Self {
            phase: state.phase,
            trust_score: state.trust_score,
            successful_interventions: state.successful_interventions,
            failed_interventions: state.failed_interventions,
            rejected_attempts: state.rejected_attempts,
            consecutive_successes: state.consecutive_successes,
        }
    }
}

impl From<TrustStateSnapshot> for TrustState {
    fn from(snapshot: TrustStateSnapshot) -> Self {
        Self {
            phase: snapshot.phase,
            trust_score: snapshot.trust_score,
            successful_interventions: snapshot.successful_interventions,
            failed_interventions: snapshot.failed_interventions,
            rejected_attempts: snapshot.rejected_attempts,
            consecutive_successes: snapshot.consecutive_successes,
        }
    }
}

/// 信任管理器
pub struct TrustManager {
    state: TrustState,
    trust_threshold: f64,
    allowed_targets: Vec<String>,
    blocked_targets: Vec<String>,
    state_file: Option<std::path::PathBuf>,
}

impl TrustManager {
    pub fn new(phase: SymbiosisPhase, threshold: f64, allowed_targets: Vec<String>) -> Self {
        Self {
            state: TrustState::new(phase),
            trust_threshold: threshold,
            allowed_targets,
            blocked_targets: Vec::new(),
            state_file: None,
        }
    }

    /// 创建带持久化的 TrustManager
    pub fn with_persistence(
        phase: SymbiosisPhase,
        threshold: f64,
        allowed_targets: Vec<String>,
        state_file: &Path,
    ) -> Self {
        let mut manager = Self::new(phase, threshold, allowed_targets);
        manager.state_file = Some(state_file.to_path_buf());
        // Try to load existing state
        if let Ok(loaded) = manager.load_state() {
            tracing::info!(phase = ?loaded.phase, "loaded persisted trust state");
            manager.state = loaded;
        }
        manager
    }

    /// 保存状态到磁盘
    pub fn save_state(&self) -> MigiResult<()> {
        let Some(ref path) = self.state_file else {
            return Ok(()); // No persistence configured
        };

        let snapshot = TrustStateSnapshot::from(&self.state);
        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| {
            MigiError::TrustViolation(format!("failed to serialize trust state: {e}"))
        })?;

        // Write to temp file first, then rename (atomic on most filesystems)
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, json)
            .map_err(|e| MigiError::TrustViolation(format!("failed to write trust state: {e}")))?;
        std::fs::rename(&temp_path, path).map_err(|e| {
            MigiError::TrustViolation(format!("failed to persist trust state: {e}"))
        })?;

        Ok(())
    }

    /// 从磁盘加载状态
    fn load_state(&self) -> MigiResult<TrustState> {
        let Some(ref path) = self.state_file else {
            return Err(MigiError::Config("no state file configured".into()));
        };

        if !path.exists() {
            return Err(MigiError::Config("state file does not exist".into()));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| MigiError::TrustViolation(format!("failed to read trust state: {e}")))?;

        let snapshot: TrustStateSnapshot = serde_json::from_str(&content).map_err(|e| {
            MigiError::TrustViolation(format!("failed to deserialize trust state: {e}"))
        })?;

        Ok(snapshot.into())
    }

    /// 设置黑名单
    pub fn set_blocked_targets(&mut self, targets: Vec<String>) {
        self.blocked_targets = targets;
    }

    /// 添加单个黑名单目标
    pub fn add_blocked_target(&mut self, target: &str) {
        if !self.blocked_targets.contains(&target.to_string()) {
            self.blocked_targets.push(target.to_string());
        }
    }

    pub fn authorize(&self, intervention: &Intervention) -> MigiResult<()> {
        // Check blacklist first
        if self.blocked_targets.contains(&intervention.target) {
            tracing::warn!(
                target = %intervention.target,
                "intervention target is in blocked list"
            );
            return Err(MigiError::TrustViolation(format!(
                "target '{}' is in the blocked intervention targets list",
                intervention.target,
            )));
        }

        // Check whitelist
        if !self.allowed_targets.is_empty() && !self.allowed_targets.contains(&intervention.target)
        {
            tracing::warn!(
                target = %intervention.target,
                "intervention target not in allowed list"
            );
            return Err(MigiError::TrustViolation(format!(
                "target '{}' not in allowed intervention targets",
                intervention.target,
            )));
        }

        match &intervention.action {
            Action::Diagnose { .. } => Ok(()),
            Action::Suggest { .. } => {
                if self.state.phase.can_suggest() {
                    Ok(())
                } else {
                    Err(MigiError::TrustViolation(
                        "current phase does not allow suggestions".into(),
                    ))
                }
            }
            Action::Hotfix { .. } | Action::Isolate { .. } | Action::Reconfigure { .. } => {
                if self.state.phase.can_write_isolated() {
                    Ok(())
                } else {
                    Err(MigiError::TrustViolation(
                        "current phase does not allow write operations".into(),
                    ))
                }
            }
            Action::EmergencyBlock { .. } => {
                if self.state.phase.can_takeover() {
                    Ok(())
                } else {
                    Err(MigiError::TrustViolation(
                        "current phase does not allow emergency blocks".into(),
                    ))
                }
            }
        }
    }

    pub fn evaluate_transition(&self, model: &SystemModel) -> MigiResult<Option<SymbiosisPhase>> {
        let current = self.state.phase;
        let model_reliable = model.is_reliable(self.trust_threshold);
        let enough_data = model.observed_events >= 100;
        let high_trust = self.state.trust_score >= 0.8;
        let streak = self.state.consecutive_successes >= 10;

        let next_phase = match current {
            SymbiosisPhase::Observation => {
                if model_reliable && enough_data {
                    Some(SymbiosisPhase::Assistance)
                } else {
                    None
                }
            }
            SymbiosisPhase::Assistance => {
                if high_trust && streak && model_reliable {
                    Some(SymbiosisPhase::LocalTakeover)
                } else {
                    None
                }
            }
            SymbiosisPhase::LocalTakeover => {
                if high_trust && streak && model.prediction_accuracy >= 0.95 {
                    Some(SymbiosisPhase::ControlledTransition)
                } else {
                    None
                }
            }
            SymbiosisPhase::ControlledTransition => None,
        };

        if let Some(phase) = &next_phase {
            tracing::info!(
                from = ?current,
                to = ?phase,
                trust_score = self.state.trust_score,
                model_accuracy = model.prediction_accuracy,
                "phase transition condition met"
            );
        }

        Ok(next_phase)
    }

    pub fn transition(&mut self, new_phase: SymbiosisPhase) -> MigiResult<()> {
        let old_phase = self.state.phase;
        tracing::warn!(
            from = ?old_phase,
            to = ?new_phase,
            "PHASE TRANSITION"
        );
        self.state.phase = new_phase;
        self.state.consecutive_successes = 0;
        // Persist after transition
        let _ = self.save_state();
        Ok(())
    }

    pub fn state(&self) -> &TrustState {
        &self.state
    }

    pub fn phase(&self) -> &SymbiosisPhase {
        &self.state.phase
    }

    pub fn allowed_targets(&self) -> &[String] {
        &self.allowed_targets
    }

    pub fn blocked_targets(&self) -> &[String] {
        &self.blocked_targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SymbiosisPhase;
    use uuid::Uuid;

    fn make_intervention(action: Action, target: &str) -> Intervention {
        Intervention {
            id: Uuid::new_v4(),
            trigger: crate::intervener::InterventionTrigger::Manual("test".into()),
            target: target.to_string(),
            action,
            executed: false,
            rollbackable: false,
            rollback_action: None,
        }
    }

    #[test]
    fn test_trust_state_initial() {
        let state = TrustState::new(SymbiosisPhase::Observation);
        assert_eq!(state.trust_score, 0.0);
        assert_eq!(state.successful_interventions, 0);
        assert_eq!(state.consecutive_successes, 0);
    }

    #[test]
    fn test_trust_state_success_updates_score() {
        let mut state = TrustState::new(SymbiosisPhase::Observation);
        state.record_success();
        assert_eq!(state.trust_score, 1.0);
        assert_eq!(state.consecutive_successes, 1);

        state.record_success();
        assert_eq!(state.trust_score, 1.0);
        assert_eq!(state.consecutive_successes, 2);
    }

    #[test]
    fn test_trust_state_failure_resets_streak() {
        let mut state = TrustState::new(SymbiosisPhase::Observation);
        state.record_success();
        state.record_success();
        state.record_failure();
        assert_eq!(state.consecutive_successes, 0);
        assert!((state.trust_score - 0.6667).abs() < 0.001); // 2 success, 1 failure = 2/3
    }

    #[test]
    fn test_authorize_diagnose_always_allowed() {
        let manager = TrustManager::new(SymbiosisPhase::Observation, 0.05, vec![]);
        let intervention = make_intervention(
            Action::Diagnose {
                command: "echo hi".into(),
            },
            "any",
        );
        assert!(manager.authorize(&intervention).is_ok());
    }

    #[test]
    fn test_authorize_suggest_observation_rejected() {
        let manager = TrustManager::new(SymbiosisPhase::Observation, 0.05, vec![]);
        let intervention = make_intervention(
            Action::Suggest {
                suggestion: "restart service".into(),
            },
            "api",
        );
        assert!(manager.authorize(&intervention).is_err());
    }

    #[test]
    fn test_authorize_suggest_assistance_allowed() {
        let manager = TrustManager::new(SymbiosisPhase::Assistance, 0.05, vec!["api".to_string()]);
        let intervention = make_intervention(
            Action::Suggest {
                suggestion: "restart service".into(),
            },
            "api",
        );
        assert!(manager.authorize(&intervention).is_ok());
    }

    #[test]
    fn test_authorize_hotfix_observation_rejected() {
        let manager = TrustManager::new(SymbiosisPhase::Observation, 0.05, vec!["db".to_string()]);
        let intervention = make_intervention(
            Action::Hotfix {
                patch: "fix".into(),
            },
            "db",
        );
        assert!(manager.authorize(&intervention).is_err());
    }

    #[test]
    fn test_authorize_hotfix_local_takeover_allowed() {
        let manager =
            TrustManager::new(SymbiosisPhase::LocalTakeover, 0.05, vec!["db".to_string()]);
        let intervention = make_intervention(
            Action::Hotfix {
                patch: "fix".into(),
            },
            "db",
        );
        assert!(manager.authorize(&intervention).is_ok());
    }

    #[test]
    fn test_authorize_blocked_target() {
        let mut manager =
            TrustManager::new(SymbiosisPhase::LocalTakeover, 0.05, vec!["db".to_string()]);
        manager.add_blocked_target("auth");
        let intervention = make_intervention(
            Action::Hotfix {
                patch: "fix".into(),
            },
            "auth",
        );
        assert!(manager.authorize(&intervention).is_err());
    }

    #[test]
    fn test_authorize_target_not_in_whitelist() {
        let manager =
            TrustManager::new(SymbiosisPhase::LocalTakeover, 0.05, vec!["db".to_string()]);
        let intervention = make_intervention(
            Action::Hotfix {
                patch: "fix".into(),
            },
            "cache",
        );
        assert!(manager.authorize(&intervention).is_err());
    }

    #[test]
    fn test_transition_observation_to_assistance() {
        let manager = TrustManager::new(SymbiosisPhase::Observation, 0.05, vec![]);
        let mut model = SystemModel::new();
        model.observed_events = 100;
        model.prediction_accuracy = 0.96; // >= 1.0 - 0.05
        let result = manager.evaluate_transition(&model).unwrap();
        assert_eq!(result, Some(SymbiosisPhase::Assistance));
    }

    #[test]
    fn test_transition_not_enough_data() {
        let manager = TrustManager::new(SymbiosisPhase::Observation, 0.05, vec![]);
        let mut model = SystemModel::new();
        model.observed_events = 50;
        model.prediction_accuracy = 0.96;
        let result = manager.evaluate_transition(&model).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_transition_assistance_to_local_takeover() {
        let mut manager =
            TrustManager::new(SymbiosisPhase::Assistance, 0.05, vec!["db".to_string()]);
        // Simulate 10 consecutive successes
        for _ in 0..10 {
            manager.state.record_success();
        }
        let mut model = SystemModel::new();
        model.observed_events = 200;
        model.prediction_accuracy = 0.96;
        let result = manager.evaluate_transition(&model).unwrap();
        assert_eq!(result, Some(SymbiosisPhase::LocalTakeover));
    }

    #[test]
    fn test_transition_local_takeover_to_controlled() {
        let mut manager =
            TrustManager::new(SymbiosisPhase::LocalTakeover, 0.05, vec!["db".to_string()]);
        for _ in 0..10 {
            manager.state.record_success();
        }
        let mut model = SystemModel::new();
        model.observed_events = 500;
        model.prediction_accuracy = 0.96; // >= 0.95
        let result = manager.evaluate_transition(&model).unwrap();
        assert_eq!(result, Some(SymbiosisPhase::ControlledTransition));
    }

    #[test]
    fn test_no_transition_from_controlled() {
        let mut manager = TrustManager::new(SymbiosisPhase::ControlledTransition, 0.05, vec![]);
        for _ in 0..100 {
            manager.state.record_success();
        }
        let mut model = SystemModel::new();
        model.observed_events = 1000;
        model.prediction_accuracy = 0.99;
        let result = manager.evaluate_transition(&model).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_transition_resets_consecutive_successes() {
        let mut manager = TrustManager::new(SymbiosisPhase::Observation, 0.05, vec![]);
        for _ in 0..5 {
            manager.state.record_success();
        }
        assert_eq!(manager.state().consecutive_successes, 5);
        manager.transition(SymbiosisPhase::Assistance).unwrap();
        assert_eq!(manager.state().consecutive_successes, 0);
        assert_eq!(manager.state().phase, SymbiosisPhase::Assistance);
    }

    #[test]
    fn test_failure_does_not_downgrade_phase() {
        let mut manager =
            TrustManager::new(SymbiosisPhase::LocalTakeover, 0.05, vec!["db".to_string()]);
        manager.state.record_failure();
        let model = SystemModel::new();
        let result = manager.evaluate_transition(&model).unwrap();
        // Should return None, NOT a lower phase
        assert_eq!(result, None);
        assert_eq!(manager.state().phase, SymbiosisPhase::LocalTakeover);
    }

    #[test]
    fn test_state_persistence() {
        let temp_dir = std::env::temp_dir();
        let state_file = temp_dir.join("migi_test_trust_state.json");
        // Clean up first
        let _ = std::fs::remove_file(&state_file);

        {
            let mut manager = TrustManager::with_persistence(
                SymbiosisPhase::Assistance,
                0.05,
                vec!["db".to_string()],
                &state_file,
            );
            manager.state.record_success();
            manager.save_state().unwrap();
        }

        // Reload
        let manager = TrustManager::with_persistence(
            SymbiosisPhase::Observation, // default, should be overridden
            0.05,
            vec!["db".to_string()],
            &state_file,
        );
        assert_eq!(manager.state().phase, SymbiosisPhase::Assistance);
        assert_eq!(manager.state().successful_interventions, 1);

        // Cleanup
        let _ = std::fs::remove_file(&state_file);
    }
}
