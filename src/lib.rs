//! Migi — 共生型 AI Agent
//!
//! "寄生"而不"接管"。观察宿主系统，学习其行为模式，
//! 在必要时局部介入，最终实现受控相变。
//!
//! 架构:
//! ```text
//! Host System ──────┐
//!                    │ (side-channel observation)
//!                    ▼
//!              ┌──────────┐
//!              │ Observer  │  感知层：静默观察数据流
//!              └─────┬────┘
//!                    │ (events)
//!              ┌─────▼────┐
//!              │  Learner  │  认知层：构建系统内部模型
//!              └─────┬────┘
//!                    │ (predictions + confidence)
//!              ┌─────▼────┐
//!              │ Intervener│  行动层：战术接管与变形
//!              └─────┬────┘
//!                    │ (approval request)
//!              ┌─────▼────┐
//!              │   Trust   │  信任层：控制权与边界管理
//!              └──────────┘
//! ```

pub mod config;
pub mod error;
pub mod intervener;
pub mod learner;
pub mod observer;
pub mod sandbox;
pub mod secrets;
pub mod trust;

pub use error::{MigiError, MigiResult};
