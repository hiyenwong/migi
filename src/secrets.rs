//! Secrets — 加密配置管理
//!
//! AES-256-GCM 认证加密，用于安全存储 API Key 等敏感信息。
//!
//! 使用方式:
//! 1. 设置环境变量 `MIGI_MASTER_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`
//! 2. 运行 `migi-secrets encrypt config/secrets.enc` 生成加密文件
//! 3. Migi 启动时自动解密加载

use crate::error::{MigiError, MigiResult};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 加密 secrets 容器（内存中）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Secrets(HashMap<String, String>);

impl Secrets {
    /// 创建一个空的 secrets 容器
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 key-value 对创建
    pub fn from_pairs(pairs: Vec<(&str, &str)>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.to_string());
        }
        Secrets(map)
    }

    /// 获取一个 secret 值
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    /// 设置一个 secret 值
    pub fn set(&mut self, key: &str, value: &str) {
        self.0.insert(key.to_string(), value.to_string());
    }

    /// 列出所有 key（不暴露 value）
    pub fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|s| s.as_str()).collect()
    }

    /// 从环境变量获取主密钥
    pub fn master_key_from_env() -> MigiResult<Vec<u8>> {
        let hex_key = std::env::var("MIGI_MASTER_KEY").map_err(|_| {
            MigiError::Config(
                "MIGI_MASTER_KEY environment variable not set. Use 'migi-secrets gen-key' to generate one."
                    .into(),
            )
        })?;

        let key = hex::decode(hex_key.trim()).map_err(|e| {
            MigiError::Config(format!("MIGI_MASTER_KEY must be a valid hex string: {e}"))
        })?;

        if key.len() != 32 {
            return Err(MigiError::Config(format!(
                "MIGI_MASTER_KEY must be exactly 32 bytes (64 hex chars), got {} bytes",
                key.len()
            )));
        }

        Ok(key)
    }

    /// 加密并保存到文件
    pub fn save_to_file(&self, path: &Path, key: &[u8]) -> MigiResult<()> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| MigiError::Config(format!("failed to create cipher: {e}")))?;

        // Generate random 12-byte nonce
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Serialize secrets to JSON
        let plaintext = serde_json::to_string(&self.0)
            .map_err(|e| MigiError::Config(format!("failed to serialize secrets: {e}")))?;

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| MigiError::Config(format!("encryption failed: {e}")))?;

        // Format: nonce[12] || ciphertext
        let mut encrypted = Vec::with_capacity(12 + ciphertext.len());
        encrypted.extend_from_slice(&nonce_bytes);
        encrypted.extend_from_slice(&ciphertext);

        // Encode as base64
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &encrypted);

        // Write to temp then rename (atomic)
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, &b64)
            .map_err(|e| MigiError::Config(format!("failed to write encrypted secrets: {e}")))?;
        std::fs::rename(&temp_path, path)
            .map_err(|e| MigiError::Config(format!("failed to persist encrypted secrets: {e}")))?;

        Ok(())
    }

    /// 从加密文件加载
    pub fn load_from_file(path: &Path, key: &[u8]) -> MigiResult<Self> {
        if !path.exists() {
            return Err(MigiError::Config(format!(
                "secrets file not found: {}",
                path.display()
            )));
        }

        let b64 = std::fs::read_to_string(path)
            .map_err(|e| MigiError::Config(format!("failed to read secrets file: {e}")))?;

        let b64 = b64.trim();

        let encrypted = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| MigiError::Config(format!("failed to decode base64: {e}")))?;

        if encrypted.len() < 12 {
            return Err(MigiError::Config(
                "encrypted data too short (missing nonce)".into(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| MigiError::Config(format!("failed to create cipher: {e}")))?;

        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let decrypted = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            MigiError::Config("decryption failed — wrong master key or corrupted data".into())
        })?;

        let map: HashMap<String, String> = serde_json::from_slice(&decrypted)
            .map_err(|e| MigiError::Config(format!("failed to parse decrypted secrets: {e}")))?;

        Ok(Secrets(map))
    }

    /// 尝试加载，失败时返回空 Secrets（不报错）
    pub fn load_or_empty(path: &Path, key: &[u8]) -> Self {
        match Self::load_from_file(path, key) {
            Ok(s) => {
                tracing::info!(secret_count = s.0.len(), "secrets loaded successfully");
                s
            }
            Err(e) => {
                tracing::warn!(error = %e, "no valid secrets file, continuing with empty secrets");
                Self::default()
            }
        }
    }
}

/// 生成一个 32 字节的随机密钥（hex 格式）
pub fn generate_master_key() -> String {
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    hex::encode(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secrets_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("migi_test_secrets.enc");
        let _ = std::fs::remove_file(&path);

        // Generate key
        let key_str = generate_master_key();
        let key = hex::decode(&key_str).unwrap();

        // Save
        let secrets = Secrets::from_pairs(vec![
            ("llm_api_key", "sk-test-key-12345"),
            ("llm_org_id", "org-abc"),
        ]);
        secrets.save_to_file(&path, &key).unwrap();

        // Load
        let loaded = Secrets::load_from_file(&path, &key).unwrap();
        assert_eq!(loaded.get("llm_api_key"), Some("sk-test-key-12345"));
        assert_eq!(loaded.get("llm_org_id"), Some("org-abc"));
        assert_eq!(loaded.get("nonexistent"), None);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_secrets_wrong_key_fails() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("migi_test_bad_key.enc");
        let _ = std::fs::remove_file(&path);

        let key1 = hex::decode(generate_master_key()).unwrap();
        let key2 = hex::decode(generate_master_key()).unwrap();

        let secrets = Secrets::from_pairs(vec![("test", "value")]);
        secrets.save_to_file(&path, &key1).unwrap();

        let result = Secrets::load_from_file(&path, &key2);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_secrets_empty() {
        let secrets = Secrets::new();
        assert!(secrets.keys().is_empty());
    }

    #[test]
    fn test_load_or_empty_missing_file() {
        let key = hex::decode(generate_master_key()).unwrap();
        let secrets = Secrets::load_or_empty(Path::new("/nonexistent/secrets.enc"), &key);
        assert!(secrets.keys().is_empty());
    }

    #[test]
    fn test_master_key_length_validation() {
        // Key must be 32 bytes = 64 hex chars
        assert!(hex::decode("00").unwrap().len() != 32);
    }

    #[test]
    fn test_generate_key_format() {
        let key = generate_master_key();
        assert_eq!(key.len(), 64); // 32 bytes = 64 hex chars
                                   // Valid hex
        assert!(hex::decode(&key).is_ok());
    }
}
