//! Intervener — 行动层
//!
//! "小右变形为刀刃"：当系统遭遇威胁或 Learner 检测到异常时，
//! 在信任边界允许的范围内进行局部介入。
//!
//! 介入类型:
//! - 防御性：阻断异常请求、隔离故障模块
//! - 修复性：热修复已知模式的 bug
//! - 优化性：动态调整配置参数
//!
//! 核心原则：
//! 1. 只在 Trust 模块授权的范围内行动
//! 2. 所有行动必须可回滚
//! 3. 行动后必须生成审计日志

use crate::error::{MigiError, MigiResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 介入动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intervention {
    /// 唯一标识
    pub id: Uuid,
    /// 触发原因
    pub trigger: InterventionTrigger,
    /// 目标子系统
    pub target: String,
    /// 动作类型
    pub action: Action,
    /// 是否已执行
    pub executed: bool,
    /// 是否可回滚
    pub rollbackable: bool,
    /// 回滚指令
    pub rollback_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionTrigger {
    /// Learner 预测到异常
    PredictedAnomaly,
    /// 检测到真实异常事件
    DetectedAnomaly,
    /// 宿主请求协助
    HostRequest,
    /// 定期健康检查
    ScheduledCheck,
    /// 手动触发（master 指令）
    Manual(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// 只读诊断（安全，任何阶段都允许）
    Diagnose { command: String },
    /// 提供建议（不直接执行）
    Suggest { suggestion: String },
    /// 热修复（需要局部接管权限）
    Hotfix { patch: String },
    /// 隔离（需要局部接管权限）
    Isolate { target: String },
    /// 配置调整（需要局部接管权限）
    Reconfigure { key: String, value: String },
    /// 紧急阻断（需要相变权限）
    EmergencyBlock { reason: String },
}

/// 介入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionResult {
    pub intervention_id: Uuid,
    pub success: bool,
    pub output: String,
    pub rollback_needed: bool,
}

/// Intervener trait
#[async_trait]
pub trait InterventionStrategy: Send + Sync {
    /// 执行介入动作
    async fn execute(&self, intervention: &Intervention) -> MigiResult<InterventionResult>;

    /// 回滚介入动作
    async fn rollback(&self, intervention_id: Uuid) -> MigiResult<()>;
}

/// 介入执行器
#[derive(Default)]
pub struct Intervener {
    strategies: Vec<Box<dyn InterventionStrategy>>,
    history: Vec<InterventionResult>,
}

impl Intervener {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_strategy(&mut self, strategy: impl InterventionStrategy + 'static) {
        self.strategies.push(Box::new(strategy));
    }

    /// 尝试执行介入（需经过 Trust 层授权检查）
    pub async fn attempt(&self, intervention: &Intervention) -> MigiResult<InterventionResult> {
        tracing::info!(
            id = %intervention.id,
            action = ?intervention.action,
            target = %intervention.target,
            "attempting intervention"
        );

        if self.strategies.is_empty() {
            return Err(MigiError::Intervener(
                "no intervention strategies registered".into(),
            ));
        }

        self.strategies[0].execute(intervention).await
    }

    pub fn history(&self) -> &[InterventionResult] {
        &self.history
    }
}
