//! Trust — 信任层
//!
//! "新一与小右之间的边界"：管理 Agent 的操作权限，
//! 决定是否允许介入、何时可以相变到更高阶段。
//!
//! 核心机制:
//! - 操作白名单（哪些目标可以被介入）
//! - 信任评分（基于模型准确率和介入历史）
//! - 相变门控（满足条件时提升共生阶段）

use crate::config::SymbiosisPhase;
use crate::error::{MigiError, MigiResult};
use crate::intervener::{Action, Intervention};
use crate::learner::SystemModel;
use serde::{Deserialize, Serialize};

/// 信任状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustState {
    /// 当前共生阶段
    pub phase: SymbiosisPhase,
    /// 信任评分（0..1，越高越可信）
    pub trust_score: f64,
    /// 成功介入次数
    pub successful_interventions: u64,
    /// 失败介入次数
    pub failed_interventions: u64,
    /// 被拒绝的介入尝试次数
    pub rejected_attempts: u64,
    /// 连续成功次数（用于相变判断）
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

    /// 记录一次成功的介入
    pub fn record_success(&mut self) {
        self.successful_interventions += 1;
        self.consecutive_successes += 1;
        self.update_score();
    }

    /// 记录一次失败的介入
    pub fn record_failure(&mut self) {
        self.failed_interventions += 1;
        self.consecutive_successes = 0;
        self.update_score();
    }

    /// 记录一次被拒绝的介入
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

/// 信任管理器
pub struct TrustManager {
    state: TrustState,
    trust_threshold: f64,
    allowed_targets: Vec<String>,
}

impl TrustManager {
    pub fn new(phase: SymbiosisPhase, threshold: f64, allowed_targets: Vec<String>) -> Self {
        Self {
            state: TrustState::new(phase),
            trust_threshold: threshold,
            allowed_targets,
        }
    }

    /// 检查介入是否被授权
    pub fn authorize(&self, intervention: &Intervention) -> MigiResult<()> {
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

    /// 评估是否满足相变条件
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

    /// 执行相变
    pub fn transition(&mut self, new_phase: SymbiosisPhase) -> MigiResult<()> {
        tracing::warn!(
            from = ?self.state.phase,
            to = ?new_phase,
            "PHASE TRANSITION"
        );
        self.state.phase = new_phase;
        self.state.consecutive_successes = 0;
        Ok(())
    }

    pub fn state(&self) -> &TrustState {
        &self.state
    }
}
