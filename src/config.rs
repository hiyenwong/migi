/// Configuration for Migi agent
use serde::{Deserialize, Serialize};

/// 共生阶段 — 定义了 Agent 当前的信任等级和操作权限
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SymbiosisPhase {
    /// 阶段 1: 观察期 — 只读权限，学习系统行为
    #[default]
    Observation,
    /// 阶段 2: 辅助期 — 提供建议，自动修复已知模式
    Assistance,
    /// 阶段 3: 局部接管 — 在隔离环境中获得写权限
    LocalTakeover,
    /// 阶段 4: 受控相变 — 逐步扩大接管范围
    ControlledTransition,
}

impl SymbiosisPhase {
    /// 判断是否可以读取宿主数据
    pub fn can_read(&self) -> bool {
        true // 所有阶段都有读权限
    }

    /// 判断是否可以提供建议
    pub fn can_suggest(&self) -> bool {
        matches!(
            self,
            Self::Assistance | Self::LocalTakeover | Self::ControlledTransition
        )
    }

    /// 判断是否可以写入（在隔离环境中）
    pub fn can_write_isolated(&self) -> bool {
        matches!(self, Self::LocalTakeover | Self::ControlledTransition)
    }

    /// 判断是否可以完全接管
    pub fn can_takeover(&self) -> bool {
        matches!(self, Self::ControlledTransition)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigiConfig {
    /// Agent 名称
    pub name: String,
    /// 当前共生阶段
    pub phase: SymbiosisPhase,
    /// 宿主系统的观察端点（日志流、API、sidecar 等）
    pub host_observation_endpoints: Vec<String>,
    /// 允许介入的子系统列表（空 = 全部禁止）
    pub allowed_intervention_targets: Vec<String>,
    /// 信任阈值：模型误差低于此值时考虑相变
    pub trust_threshold: f64,
    /// 最大并发介入数
    pub max_concurrent_interventions: usize,
}

impl Default for MigiConfig {
    fn default() -> Self {
        Self {
            name: "migi".to_string(),
            phase: SymbiosisPhase::Observation,
            host_observation_endpoints: vec![],
            allowed_intervention_targets: vec![],
            trust_threshold: 0.05,
            max_concurrent_interventions: 1,
        }
    }
}
