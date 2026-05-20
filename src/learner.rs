//! Learner — 认知层
//!
//! "小右的大脑"：从 Observer 的事件流中学习宿主系统的
//! 行为模式，构建内部世界模型，并生成预测。
//!
//! 学习内容:
//! - 正常行为基线（请求模式、资源使用、错误率）
//! - 异常模式（周期性故障、级联失效前兆）
//! - 系统拓扑（模块间依赖关系、调用链）

use crate::error::MigiResult;
use crate::observer::HostEvent;
use serde::{Deserialize, Serialize};

/// 系统行为模型
///
/// 描述了 Learner 对宿主系统的理解程度。
/// 随着观察时间增长，模型逐渐精确。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemModel {
    /// 模型版本（每次更新递增）
    pub version: u64,
    /// 已观察的事件总数
    pub observed_events: u64,
    /// 模型对系统行为的预测准确率（0..1）
    pub prediction_accuracy: f64,
    /// 模型熵（越低表示模型越确定）
    pub model_entropy: f64,
    /// 已识别的子系统数量
    pub identified_subsystems: usize,
    /// 已识别的调用模式数量
    pub identified_patterns: usize,
}

impl SystemModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// 模型是否足够可靠（用于相变判断）
    pub fn is_reliable(&self, threshold: f64) -> bool {
        self.prediction_accuracy >= (1.0 - threshold)
    }
}

/// Learner trait
///
/// 定义如何从事件流中学习并更新系统模型。
/// 具体实现可以是统计模型、ML 模型、或规则引擎。
pub trait Learner: Send + Sync {
    /// 处理一批事件，更新内部模型
    fn process_events(&mut self, events: &[HostEvent]) -> MigiResult<()>;

    /// 获取当前系统模型
    fn get_model(&self) -> &SystemModel;

    /// 对即将发生的事件进行预测
    fn predict(&self) -> MigiResult<Predictions>;
}

/// 预测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predictions {
    /// 预计下一时段的事件类型分布
    pub event_distribution: Vec<(String, f64)>,
    /// 异常概率（0..1）
    pub anomaly_probability: f64,
    /// 预测置信度（0..1）
    pub confidence: f64,
}

/// 默认 Learner 实现（基于统计的简单学习器）
///
/// 阶段 1（观察期）的默认行为：
/// 统计事件频率、构建事件类型直方图、检测频率偏离。
#[derive(Default)]
pub struct StatisticalLearner {
    model: SystemModel,
    event_counts: std::collections::HashMap<String, u64>,
    total_events: u64,
}

impl StatisticalLearner {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Learner for StatisticalLearner {
    fn process_events(&mut self, events: &[HostEvent]) -> MigiResult<()> {
        for event in events {
            let key = format!("{:?}_{:?}", event.source, event.event_type);
            *self.event_counts.entry(key).or_insert(0) += 1;
            self.total_events += 1;
        }

        self.model.observed_events = self.total_events;
        self.model.version += 1;
        self.model.identified_patterns = self.event_counts.len();

        // 简单估计：数据越多，模型越可靠（对数增长）
        if self.total_events > 0 {
            self.model.prediction_accuracy =
                (self.total_events as f64).ln() / (self.total_events as f64).ln().max(10.0);
            self.model.prediction_accuracy = self.model.prediction_accuracy.min(1.0);
            self.model.model_entropy = 1.0 - self.model.prediction_accuracy;
        }

        Ok(())
    }

    fn get_model(&self) -> &SystemModel {
        &self.model
    }

    fn predict(&self) -> MigiResult<Predictions> {
        let total = self.total_events.max(1) as f64;
        let distribution: Vec<(String, f64)> = self
            .event_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total))
            .collect();

        Ok(Predictions {
            event_distribution: distribution,
            anomaly_probability: 0.0,
            confidence: self.model.prediction_accuracy,
        })
    }
}
