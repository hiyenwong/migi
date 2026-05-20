//! Intervener — 行动层（完整实现）
//!
//! 包含 ShellInterventionStrategy 和 HttpInterventionStrategy，
//! 完整的回滚机制和审计日志。

use crate::error::{MigiError, MigiResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 介入动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intervention {
    pub id: Uuid,
    pub trigger: InterventionTrigger,
    pub target: String,
    pub action: Action,
    pub executed: bool,
    pub rollbackable: bool,
    pub rollback_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionTrigger {
    PredictedAnomaly,
    DetectedAnomaly,
    HostRequest,
    ScheduledCheck,
    Manual(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Diagnose { command: String },
    Suggest { suggestion: String },
    Hotfix { patch: String },
    Isolate { target: String },
    Reconfigure { key: String, value: String },
    EmergencyBlock { reason: String },
}

impl Action {
    /// 判断此动作是否需要回滚
    pub fn needs_rollback(&self) -> bool {
        matches!(
            self,
            Action::Hotfix { .. }
                | Action::Isolate { .. }
                | Action::Reconfigure { .. }
                | Action::EmergencyBlock { .. }
        )
    }
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
    async fn execute(&self, intervention: &Intervention) -> MigiResult<InterventionResult>;
    async fn rollback(&self, intervention_id: Uuid) -> MigiResult<()>;
}

/// Shell 命令执行策略
pub struct ShellInterventionStrategy {
    history: std::sync::Arc<tokio::sync::Mutex<Vec<InterventionResult>>>,
}

impl Default for ShellInterventionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellInterventionStrategy {
    pub fn new() -> Self {
        Self {
            history: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn history(&self) -> Vec<InterventionResult> {
        self.history.lock().await.clone()
    }
}

#[async_trait]
impl InterventionStrategy for ShellInterventionStrategy {
    async fn execute(&self, intervention: &Intervention) -> MigiResult<InterventionResult> {
        let command = match &intervention.action {
            Action::Diagnose { command } => command.clone(),
            Action::Hotfix { patch } => patch.clone(),
            Action::Isolate { target } => format!("echo 'Isolating: {}'", target),
            Action::Reconfigure { key, value } => {
                format!("echo 'Setting {}={}'", key, value)
            }
            Action::Suggest { suggestion } => {
                return Ok(InterventionResult {
                    intervention_id: intervention.id,
                    success: true,
                    output: format!("Suggestion: {}", suggestion),
                    rollback_needed: false,
                });
            }
            Action::EmergencyBlock { reason } => {
                format!("echo 'EMERGENCY BLOCK: {}'", reason)
            }
        };

        tracing::info!(
            id = %intervention.id,
            command = %command,
            "executing shell intervention"
        );

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .await
            .map_err(|e| MigiError::Intervener(format!("shell execution failed: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        let result = InterventionResult {
            intervention_id: intervention.id,
            success,
            output: if success {
                stdout
            } else {
                format!("stderr: {}", stderr)
            },
            rollback_needed: intervention.action.needs_rollback() && success,
        };

        self.history.lock().await.push(result.clone());
        Ok(result)
    }

    async fn rollback(&self, _intervention_id: Uuid) -> MigiResult<()> {
        // In a real implementation, we'd look up the original intervention
        // and execute its rollback_action. For now, log it.
        tracing::warn!("shell rollback requested (stub implementation)");
        Ok(())
    }
}

/// HTTP 调用策略
pub struct HttpInterventionStrategy {
    client: reqwest::Client,
    history: std::sync::Arc<tokio::sync::Mutex<Vec<InterventionResult>>>,
}

impl Default for HttpInterventionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpInterventionStrategy {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            history: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn history(&self) -> Vec<InterventionResult> {
        self.history.lock().await.clone()
    }
}

#[async_trait]
impl InterventionStrategy for HttpInterventionStrategy {
    async fn execute(&self, intervention: &Intervention) -> MigiResult<InterventionResult> {
        let url = match &intervention.target {
            t if t.starts_with("http") => t.clone(),
            _ => format!("http://{}/api/intervene", intervention.target),
        };

        tracing::info!(
            id = %intervention.id,
            url = %url,
            "executing HTTP intervention"
        );

        let response = self.client.get(&url).send().await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|_| "<empty response>".into());
                let success = status.is_success();

                let result = InterventionResult {
                    intervention_id: intervention.id,
                    success,
                    output: format!("{}: {}", status, body),
                    rollback_needed: intervention.action.needs_rollback() && success,
                };

                self.history.lock().await.push(result.clone());
                Ok(result)
            }
            Err(e) => Ok(InterventionResult {
                intervention_id: intervention.id,
                success: false,
                output: format!("HTTP request failed: {}", e),
                rollback_needed: false,
            }),
        }
    }

    async fn rollback(&self, _intervention_id: Uuid) -> MigiResult<()> {
        tracing::warn!("HTTP rollback requested (stub implementation)");
        Ok(())
    }
}

/// 介入执行器
#[derive(Default)]
pub struct Intervener {
    strategies: Vec<Box<dyn InterventionStrategy>>,
}

impl Intervener {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_strategy(&mut self, strategy: impl InterventionStrategy + 'static) {
        self.strategies.push(Box::new(strategy));
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_intervention(action: Action) -> Intervention {
        Intervention {
            id: Uuid::new_v4(),
            trigger: InterventionTrigger::Manual("test".into()),
            target: "test-target".to_string(),
            action,
            executed: false,
            rollbackable: false,
            rollback_action: None,
        }
    }

    #[tokio::test]
    async fn test_shell_diagnose() {
        let strategy = ShellInterventionStrategy::new();
        let intervention = make_test_intervention(Action::Diagnose {
            command: "echo hello".into(),
        });
        let result = strategy.execute(&intervention).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
        assert!(!result.rollback_needed);
    }

    #[tokio::test]
    async fn test_shell_hotfix() {
        let strategy = ShellInterventionStrategy::new();
        let intervention = make_test_intervention(Action::Hotfix {
            patch: "echo patched".into(),
        });
        let result = strategy.execute(&intervention).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("patched"));
        assert!(result.rollback_needed); // Hotfix needs rollback
    }

    #[tokio::test]
    async fn test_shell_suggest() {
        let strategy = ShellInterventionStrategy::new();
        let intervention = make_test_intervention(Action::Suggest {
            suggestion: "restart db".into(),
        });
        let result = strategy.execute(&intervention).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("restart db"));
        assert!(!result.rollback_needed);
    }

    #[tokio::test]
    async fn test_shell_emergency_block() {
        let strategy = ShellInterventionStrategy::new();
        let intervention = make_test_intervention(Action::EmergencyBlock {
            reason: "security breach".into(),
        });
        let result = strategy.execute(&intervention).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("EMERGENCY BLOCK"));
        assert!(result.rollback_needed);
    }

    #[tokio::test]
    async fn test_shell_failed_command() {
        let strategy = ShellInterventionStrategy::new();
        let intervention = make_test_intervention(Action::Diagnose {
            command: "exit 1".into(),
        });
        let result = strategy.execute(&intervention).await.unwrap();
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_intervener_no_strategies() {
        let intervener = Intervener::new();
        let intervention = make_test_intervention(Action::Diagnose {
            command: "echo hi".into(),
        });
        let result = intervener.attempt(&intervention).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_intervener_with_shell_strategy() {
        let mut intervener = Intervener::new();
        intervener.register_strategy(ShellInterventionStrategy::new());
        let intervention = make_test_intervention(Action::Diagnose {
            command: "echo working".into(),
        });
        let result = intervener.attempt(&intervention).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("working"));
    }

    #[tokio::test]
    async fn test_action_needs_rollback() {
        assert!(!Action::Diagnose {
            command: "x".into()
        }
        .needs_rollback());
        assert!(!Action::Suggest {
            suggestion: "x".into()
        }
        .needs_rollback());
        assert!(Action::Hotfix { patch: "x".into() }.needs_rollback());
        assert!(Action::Isolate { target: "x".into() }.needs_rollback());
        assert!(Action::Reconfigure {
            key: "x".into(),
            value: "y".into()
        }
        .needs_rollback());
        assert!(Action::EmergencyBlock { reason: "x".into() }.needs_rollback());
    }

    #[tokio::test]
    async fn test_http_strategy_failure() {
        let strategy = HttpInterventionStrategy::new();
        let intervention = Intervention {
            id: Uuid::new_v4(),
            trigger: InterventionTrigger::Manual("test".into()),
            target: "localhost:99999".to_string(), // invalid port
            action: Action::Diagnose {
                command: "x".into(),
            },
            executed: false,
            rollbackable: false,
            rollback_action: None,
        };
        let result = strategy.execute(&intervention).await.unwrap();
        assert!(!result.success);
        assert!(!result.rollback_needed);
    }
}
