//! Observer — 感知层
//!
//! "小右的神经末梢"：静默观察宿主系统的数据流，
//! 不干涉、不修改，只感知。
//!
//! 观察通道:
//! - 日志流 (log tailing)
//! - 系统指标 (metrics polling)
//! - 网络流量 (packet metadata)
//! - 文件变更 (filesystem events)

use crate::error::MigiResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// 宿主系统发出的事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostEvent {
    /// 事件时间戳
    pub timestamp: SystemTime,
    /// 事件来源（子系统/模块名）
    pub source: String,
    /// 事件类型
    pub event_type: EventType,
    /// 事件载荷（结构化数据）
    pub payload: serde_json::Value,
    /// 事件优先级
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// 请求进入
    RequestIn,
    /// 请求完成
    RequestComplete,
    /// 错误发生
    Error,
    /// 状态变更
    StateChange,
    /// 资源阈值告警
    ResourceAlert,
    /// 自定义事件
    Custom(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// 观察通道 trait
///
/// 每种通道实现如何从宿主系统获取数据。
/// 实现方可以是 log reader、metrics scraper、sidecar proxy 等。
#[async_trait]
pub trait ObservationChannel: Send + Sync {
    /// 通道名称（用于日志和调试）
    fn name(&self) -> &str;

    /// 启动观察（开始监听数据流）
    async fn start(&mut self) -> MigiResult<()>;

    /// 获取下一个事件（异步流）
    async fn next_event(&mut self) -> MigiResult<Option<HostEvent>>;

    /// 停止观察
    async fn stop(&mut self) -> MigiResult<()>;
}

/// 观察者核心结构
///
/// 管理多个观察通道，统一聚合事件流。
#[derive(Default)]
pub struct Observer {
    channels: Vec<Box<dyn ObservationChannel>>,
    event_count: u64,
}

impl Observer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个观察通道
    pub fn register_channel(&mut self, channel: impl ObservationChannel + 'static) {
        tracing::info!(channel = %channel.name(), "registering observation channel");
        self.channels.push(Box::new(channel));
    }

    /// 启动所有通道
    pub async fn start_all(&mut self) -> MigiResult<()> {
        for ch in &mut self.channels {
            ch.start().await?;
        }
        tracing::info!("all observation channels started");
        Ok(())
    }

    /// 轮询所有通道获取事件
    pub async fn poll_events(&mut self) -> MigiResult<Vec<HostEvent>> {
        let mut events = Vec::new();
        for ch in &mut self.channels {
            if let Some(event) = ch.next_event().await? {
                self.event_count += 1;
                events.push(event);
            }
        }
        Ok(events)
    }

    /// 已处理的事件总数
    pub fn event_count(&self) -> u64 {
        self.event_count
    }
}
