/// Configuration for Migi agent
use crate::error::{MigiError, MigiResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SymbiosisPhase {
    #[default]
    Observation,
    Assistance,
    LocalTakeover,
    ControlledTransition,
}

impl SymbiosisPhase {
    pub fn can_read(&self) -> bool {
        true
    }

    pub fn can_suggest(&self) -> bool {
        matches!(
            self,
            Self::Assistance | Self::LocalTakeover | Self::ControlledTransition
        )
    }

    pub fn can_write_isolated(&self) -> bool {
        matches!(self, Self::LocalTakeover | Self::ControlledTransition)
    }

    pub fn can_takeover(&self) -> bool {
        matches!(self, Self::ControlledTransition)
    }
}

/// 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigiConfig {
    pub name: String,
    pub phase: SymbiosisPhase,
    pub host_observation_endpoints: Vec<String>,
    pub allowed_intervention_targets: Vec<String>,
    pub trust_threshold: f64,
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

impl MigiConfig {
    /// 从 TOML 文件加载配置
    pub fn from_file(path: &Path) -> MigiResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            MigiError::Config(format!(
                "failed to read config file '{}': {e}",
                path.display()
            ))
        })?;

        let config: MigiConfig = toml::from_str(&content)
            .map_err(|e| MigiError::Config(format!("failed to parse config file: {e}")))?;

        tracing::info!(
            name = %config.name,
            phase = ?config.phase,
            endpoints = config.host_observation_endpoints.len(),
            allowed_targets = config.allowed_intervention_targets.len(),
            "configuration loaded"
        );

        Ok(config)
    }

    /// 尝试从文件加载，失败时使用默认配置
    pub fn load_or_default(path: &Path) -> Self {
        match Self::from_file(path) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!(error = %e, "falling back to default configuration");
                Self::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MigiConfig::default();
        assert_eq!(config.name, "migi");
        assert_eq!(config.phase, SymbiosisPhase::Observation);
        assert!(config.host_observation_endpoints.is_empty());
        assert!(config.allowed_intervention_targets.is_empty());
        assert_eq!(config.trust_threshold, 0.05);
        assert_eq!(config.max_concurrent_interventions, 1);
    }

    #[test]
    fn test_load_or_default_missing_file() {
        let config = MigiConfig::load_or_default(Path::new("/nonexistent/migi.toml"));
        assert_eq!(config.name, "migi");
        assert_eq!(config.phase, SymbiosisPhase::Observation);
    }

    #[test]
    fn test_load_or_default_valid_file() {
        let temp_dir = std::env::temp_dir();
        let config_file = temp_dir.join("migi_test_config.toml");
        let toml_content = r#"
name = "test-migi"
phase = "assistance"
host_observation_endpoints = ["/var/log/syslog"]
allowed_intervention_targets = ["db", "cache"]
trust_threshold = 0.1
max_concurrent_interventions = 3
"#;
        std::fs::write(&config_file, toml_content).unwrap();

        let config = MigiConfig::load_or_default(&config_file);
        assert_eq!(config.name, "test-migi");
        assert_eq!(config.phase, SymbiosisPhase::Assistance);
        assert_eq!(config.host_observation_endpoints, vec!["/var/log/syslog"]);
        assert_eq!(config.allowed_intervention_targets, vec!["db", "cache"]);
        assert_eq!(config.trust_threshold, 0.1);
        assert_eq!(config.max_concurrent_interventions, 3);

        let _ = std::fs::remove_file(&config_file);
    }

    #[test]
    fn test_from_file_invalid_toml() {
        let temp_dir = std::env::temp_dir();
        let config_file = temp_dir.join("migi_invalid.toml");
        std::fs::write(&config_file, "this is not toml {{{").unwrap();

        let result = MigiConfig::from_file(&config_file);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&config_file);
    }
}
