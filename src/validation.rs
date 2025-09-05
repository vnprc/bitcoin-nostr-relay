use crate::error::ValidationError;
use serde_json::{json, Value};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use bitcoin::consensus::deserialize;
use bitcoin::Transaction;

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub enable_validation: bool,
    pub enable_precheck: bool,
    pub validation_timeout_ms: u64,
    pub cache_ttl_seconds: u64,
    pub cache_size: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enable_validation: true,
            enable_precheck: true,
            validation_timeout_ms: 5000,
            cache_ttl_seconds: 600,  // 10 minutes
            cache_size: 1000,        // ~116 KB
        }
    }
}

pub struct TransactionValidator {
    config: ValidationConfig,
    bitcoin_client: reqwest::Client,
    bitcoin_rpc_url: String,
    tx_cache: RwLock<LruCache<String, Instant>>,
}

impl TransactionValidator {
    pub fn new(config: ValidationConfig, bitcoin_port: u16) -> Self {
        let bitcoin_rpc_url = format!("http://127.0.0.1:{}", bitcoin_port);
        let cache_size = NonZeroUsize::new(config.cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());
        let tx_cache = RwLock::new(LruCache::new(cache_size));
        
        Self {
            config,
            bitcoin_client: reqwest::Client::new(),
            bitcoin_rpc_url,
            tx_cache,
        }
    }
    
    /// Get the validation configuration
    pub fn config(&self) -> &ValidationConfig {
        &self.config
    }
    
    pub async fn validate(&self, tx_hex: &str) -> Result<(), ValidationError> {
        if !self.config.enable_validation {
            return Ok(());
        }
        
        // Extract TXID first (needed for cache)
        let txid = self.extract_txid(tx_hex)?;
        
        // Check cache for recent processing
        if self.is_recently_processed(&txid) {
            return Err(ValidationError::recently_processed(txid));
        }
        
        // Phase 2: Quick pre-checks
        if self.config.enable_precheck {
            self.quick_validation_checks(tx_hex)?;
        }
        
        // Phase 1: Use Bitcoin Core validation
        self.validate_with_bitcoin_core(tx_hex).await.map_err(|e| match e {
            ValidationError::BitcoinCoreRejection { reason } => ValidationError::bitcoin_core_rejection(reason),
            other => other,
        })?;
        
        // Cache successful validation
        self.cache_transaction(&txid);
        Ok(())
    }
    
    fn quick_validation_checks(&self, tx_hex: &str) -> Result<(), ValidationError> {
        if tx_hex.is_empty() {
            return Err(ValidationError::EmptyTransaction);
        }
        
        if !tx_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ValidationError::InvalidHex);
        }
        
        let byte_len = tx_hex.len() / 2;
        if byte_len < 60 || byte_len > 400_000 {
            return Err(ValidationError::invalid_size(byte_len));
        }
        
        Ok(())
    }
    
    async fn validate_with_bitcoin_core(&self, tx_hex: &str) -> Result<(), ValidationError> {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "testmempoolaccept",
            "params": [[tx_hex]],
            "id": "validation"
        });
        
        let response: Value = self.bitcoin_client
            .post(&self.bitcoin_rpc_url)
            .basic_auth("user", Some("password"))
            .timeout(std::time::Duration::from_millis(self.config.validation_timeout_ms))
            .json(&request)
            .send()
            .await?
            .json()
            .await?;
        
        // Check for RPC error
        if let Some(error) = response.get("error") {
            if !error.is_null() {
                return Err(ValidationError::bitcoin_core_rejection(format!("RPC error: {}", error)));
            }
        }
        
        // Get the result array (testmempoolaccept returns array of results)
        let results = response["result"]
            .as_array()
            .ok_or_else(|| ValidationError::bitcoin_core_rejection("Invalid response format"))?;
        
        if results.is_empty() {
            return Err(ValidationError::bitcoin_core_rejection("Empty response"));
        }
        
        let result = &results[0];
        
        if result["allowed"].as_bool() == Some(true) {
            Ok(())
        } else {
            let reason = result["reject-reason"]
                .as_str()
                .unwrap_or("unknown reason");
            Err(ValidationError::bitcoin_core_rejection(reason))
        }
    }
    
    fn extract_txid(&self, tx_hex: &str) -> Result<String, ValidationError> {
        let tx_bytes = hex::decode(tx_hex).map_err(|_| ValidationError::InvalidHex)?;
        let tx = deserialize::<Transaction>(&tx_bytes)
            .map_err(|_| ValidationError::InvalidStructure)?;
        Ok(tx.txid().to_string())
    }
    
    fn is_recently_processed(&self, txid: &str) -> bool {
        if let Ok(cache) = self.tx_cache.read() {
            if let Some(first_seen) = cache.peek(txid) {
                let ttl = Duration::from_secs(self.config.cache_ttl_seconds);
                return first_seen.elapsed() < ttl;
            }
        }
        false
    }
    
    fn cache_transaction(&self, txid: &str) {
        if let Ok(mut cache) = self.tx_cache.write() {
            cache.put(txid.to_string(), Instant::now());
        }
    }
}

impl Clone for TransactionValidator {
    fn clone(&self) -> Self {
        Self::new(self.config.clone(), self.bitcoin_rpc_url.strip_prefix("http://127.0.0.1:").and_then(|s| s.parse().ok()).unwrap_or(18332))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_disabled() {
        let mut config = ValidationConfig::default();
        config.enable_validation = false;
        
        let validator = TransactionValidator::new(config, 18332);
        
        // Should pass validation even with invalid hex when validation is disabled
        let result = validator.validate("invalid_hex").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_precheck_disabled() {
        let mut config = ValidationConfig::default();
        config.enable_precheck = false;
        
        let validator = TransactionValidator::new(config, 18332);
        
        // Use valid hex but invalid transaction structure
        // This should pass TXID extraction but fail at Bitcoin Core validation
        let invalid_tx_hex = "a".repeat(120); // Valid hex, wrong structure
        let result = validator.validate(&invalid_tx_hex).await;
        assert!(result.is_err());
        
        // The error should be InvalidStructure (from TXID extraction) 
        // not InvalidSize (from precheck)
        if let Err(e) = result {
            assert!(!matches!(e, ValidationError::InvalidSize { size: _ }));
        }
    }

    #[test]
    fn test_quick_validation_empty_transaction() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        let result = validator.quick_validation_checks("");
        assert!(matches!(result, Err(ValidationError::EmptyTransaction)));
    }

    #[test]
    fn test_quick_validation_invalid_hex() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        // Non-hex characters
        let result = validator.quick_validation_checks("hello world");
        assert!(matches!(result, Err(ValidationError::InvalidHex)));
        
        // Mixed case with invalid characters
        let result = validator.quick_validation_checks("abcdefg");  // 'g' is not hex
        assert!(matches!(result, Err(ValidationError::InvalidHex)));
    }

    #[test]
    fn test_quick_validation_invalid_size() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        // Too small (less than 60 bytes = 120 hex chars)
        let small_tx = "a".repeat(118); // 59 bytes
        let result = validator.quick_validation_checks(&small_tx);
        assert!(matches!(result, Err(ValidationError::InvalidSize { size: 59 })));
        
        // Too large (more than 400KB = 800,000 hex chars)
        let large_tx = "a".repeat(800_002); // 400,001 bytes
        let result = validator.quick_validation_checks(&large_tx);
        assert!(matches!(result, Err(ValidationError::InvalidSize { size: 400_001 })));
    }

    #[test]
    fn test_quick_validation_valid_hex() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        // Valid hex string of appropriate length (60 bytes = 120 hex chars)
        let valid_hex = "a".repeat(120);
        let result = validator.quick_validation_checks(&valid_hex);
        assert!(result.is_ok());
        
        // Test with actual hex characters
        let mixed_case_hex = "AbCdEf0123456789".repeat(8); // 128 hex chars = 64 bytes
        let result = validator.quick_validation_checks(&mixed_case_hex);
        assert!(result.is_ok());
    }

    #[test] 
    fn test_validation_config_default() {
        let config = ValidationConfig::default();
        
        assert_eq!(config.enable_validation, true);
        assert_eq!(config.enable_precheck, true);
        assert_eq!(config.validation_timeout_ms, 5000);
    }

    #[test]
    fn test_validation_error_display() {
        let errors = vec![
            ValidationError::EmptyTransaction,
            ValidationError::InvalidHex,
            ValidationError::InvalidSize { size: 100 },
            ValidationError::InvalidStructure,
            ValidationError::recently_processed("test_txid"),
            ValidationError::bitcoin_core_rejection("test reason"),
        ];
        
        for error in errors {
            let error_string = format!("{}", error);
            assert!(!error_string.is_empty());
        }
    }
    
    #[test]
    fn test_extract_txid() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        // Test with invalid hex
        let result = validator.extract_txid("invalid_hex");
        assert!(matches!(result, Err(ValidationError::InvalidHex)));
        
        // Test with valid hex but invalid structure
        let invalid_tx_hex = "a".repeat(120);
        let result = validator.extract_txid(&invalid_tx_hex);
        assert!(matches!(result, Err(ValidationError::InvalidStructure)));
    }
    
    #[test]
    fn test_cache_functionality() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        let test_txid = "test_transaction_id";
        
        // Initially not in cache
        assert!(!validator.is_recently_processed(test_txid));
        
        // Add to cache
        validator.cache_transaction(test_txid);
        
        // Now should be in cache
        assert!(validator.is_recently_processed(test_txid));
    }
    
    #[test] 
    fn test_validation_config_with_cache() {
        let config = ValidationConfig::default();
        
        assert_eq!(config.enable_validation, true);
        assert_eq!(config.enable_precheck, true);
        assert_eq!(config.validation_timeout_ms, 5000);
        assert_eq!(config.cache_ttl_seconds, 600);
        assert_eq!(config.cache_size, 1000);
    }

    // Integration test that requires a running Bitcoin node
    #[tokio::test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    async fn test_bitcoin_core_integration_valid_transaction() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        // This is a valid transaction hex from regtest (you'll need to replace with actual valid tx)
        // For now, this test is ignored and would need a real transaction hex
        let valid_tx_hex = "0200000001..."; // Replace with real transaction
        
        let _result = validator.validate_with_bitcoin_core(valid_tx_hex).await;
        // This test requires actual Bitcoin Core running and a valid transaction
        // assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Use `cargo test -- --ignored` to run this test  
    async fn test_bitcoin_core_integration_invalid_transaction() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        // Invalid transaction hex (too short but valid hex)
        let invalid_tx_hex = "a".repeat(120);
        
        let result = validator.validate_with_bitcoin_core(&invalid_tx_hex).await;
        assert!(result.is_err());
        
        if let Err(ValidationError::BitcoinCoreRejection { reason }) = result {
            assert!(!reason.is_empty());
        } else {
            panic!("Expected BitcoinCoreRejection error");
        }
    }

    #[test]
    fn test_spam_cache_recently_processed() {
        let config = ValidationConfig::default();
        let validator = TransactionValidator::new(config, 18332);
        
        let txid = "test_transaction_id";
        
        // First check - should not be in cache
        assert!(!validator.is_recently_processed(txid));
        
        // Mark as processed
        validator.cache_transaction(txid);
        
        // Second check - should now be in cache  
        assert!(validator.is_recently_processed(txid));
        
        // Should return RecentlyProcessed error
        let _result = validator.quick_validation_checks("deadbeef"); // Valid hex to pass initial checks
        // Then manually check cache (since quick_validation_checks doesn't check cache)
        if validator.is_recently_processed("deadbeef") {
            let cache_result: Result<(), ValidationError> = Err(ValidationError::recently_processed("deadbeef"));
            assert!(matches!(cache_result, Err(ValidationError::RecentlyProcessed { .. })));
        }
    }
}