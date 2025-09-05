use crate::validation::ValidationConfig;

/// Configuration for the Bitcoin-Nostr relay server
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Bitcoin RPC URL (e.g., "http://127.0.0.1:18332")
    pub bitcoin_rpc_url: String,
    
    /// Bitcoin RPC username
    pub bitcoin_rpc_username: String,
    
    /// Bitcoin RPC password
    pub bitcoin_rpc_password: String,
    
    /// Bitcoin RPC port for the validator
    pub bitcoin_rpc_port: u16,
    
    /// Relay identifier (unique for each relay instance)
    pub relay_id: u16,
    
    /// Port for the strfry Nostr relay connection
    pub strfry_port: u16,
    
    /// Configuration for transaction validation
    pub validation_config: ValidationConfig,
    
    /// Mempool polling interval in seconds
    pub mempool_poll_interval_secs: u64,
    
    /// Maximum number of concurrent client connections
    pub max_client_connections: usize,
    
    /// WebSocket buffer size for client connections
    pub websocket_buffer_size: usize,
}

impl RelayConfig {
    pub fn new(bitcoin_rpc_port: u16, relay_id: u16, strfry_port: u16) -> Self {
        Self {
            bitcoin_rpc_url: format!("http://127.0.0.1:{}", bitcoin_rpc_port),
            bitcoin_rpc_username: "user".to_string(),
            bitcoin_rpc_password: "password".to_string(),
            bitcoin_rpc_port,
            relay_id,
            strfry_port,
            validation_config: ValidationConfig::default(),
            mempool_poll_interval_secs: 2,
            max_client_connections: 1000,
            websocket_buffer_size: 100,
        }
    }
    
    /// Create a regtest configuration
    pub fn regtest(relay_id: u16) -> Self {
        let bitcoin_port = if relay_id == 1 { 18332 } else { 18444 };
        let strfry_port = if relay_id == 1 { 7777 } else { 7778 };
        Self::new(bitcoin_port, relay_id, strfry_port)
    }
    
    /// Create a testnet4 configuration
    pub fn testnet4(relay_id: u16) -> Self {
        let bitcoin_port = if relay_id == 1 { 48330 } else { 48350 };
        let strfry_port = if relay_id == 1 { 7777 } else { 7778 };
        Self::new(bitcoin_port, relay_id, strfry_port)
    }
    
    /// Set custom Bitcoin RPC credentials
    pub fn with_bitcoin_auth(mut self, username: String, password: String) -> Self {
        self.bitcoin_rpc_username = username;
        self.bitcoin_rpc_password = password;
        self
    }
    
    /// Set custom validation configuration
    pub fn with_validation_config(mut self, validation_config: ValidationConfig) -> Self {
        self.validation_config = validation_config;
        self
    }
    
    /// Set custom mempool polling interval
    pub fn with_mempool_poll_interval(mut self, seconds: u64) -> Self {
        self.mempool_poll_interval_secs = seconds;
        self
    }
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self::regtest(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_config_new() {
        let config = RelayConfig::new(18332, 1, 7777);
        
        assert_eq!(config.bitcoin_rpc_url, "http://127.0.0.1:18332");
        assert_eq!(config.bitcoin_rpc_username, "user");
        assert_eq!(config.bitcoin_rpc_password, "password");
        assert_eq!(config.bitcoin_rpc_port, 18332);
        assert_eq!(config.relay_id, 1);
        assert_eq!(config.strfry_port, 7777);
        assert_eq!(config.mempool_poll_interval_secs, 2);
        assert_eq!(config.max_client_connections, 1000);
        assert_eq!(config.websocket_buffer_size, 100);
    }

    #[test]
    fn test_relay_config_regtest() {
        let config1 = RelayConfig::regtest(1);
        assert_eq!(config1.bitcoin_rpc_port, 18332);
        assert_eq!(config1.strfry_port, 7777);
        assert_eq!(config1.relay_id, 1);
        
        let config2 = RelayConfig::regtest(2);
        assert_eq!(config2.bitcoin_rpc_port, 18444);
        assert_eq!(config2.strfry_port, 7778);
        assert_eq!(config2.relay_id, 2);
        
        // Test URL construction
        assert_eq!(config1.bitcoin_rpc_url, "http://127.0.0.1:18332");
        assert_eq!(config2.bitcoin_rpc_url, "http://127.0.0.1:18444");
    }

    #[test]
    fn test_relay_config_testnet4() {
        let config1 = RelayConfig::testnet4(1);
        assert_eq!(config1.bitcoin_rpc_port, 48330);
        assert_eq!(config1.strfry_port, 7777);
        assert_eq!(config1.relay_id, 1);
        
        let config2 = RelayConfig::testnet4(2);
        assert_eq!(config2.bitcoin_rpc_port, 48350);
        assert_eq!(config2.strfry_port, 7778);
        assert_eq!(config2.relay_id, 2);
        
        // Test URL construction
        assert_eq!(config1.bitcoin_rpc_url, "http://127.0.0.1:48330");
        assert_eq!(config2.bitcoin_rpc_url, "http://127.0.0.1:48350");
    }

    #[test]
    fn test_relay_config_default() {
        let config = RelayConfig::default();
        
        // Default should be regtest relay 1
        assert_eq!(config.bitcoin_rpc_port, 18332);
        assert_eq!(config.strfry_port, 7777);
        assert_eq!(config.relay_id, 1);
        assert_eq!(config.bitcoin_rpc_url, "http://127.0.0.1:18332");
    }

    #[test]
    fn test_relay_config_clone() {
        let config1 = RelayConfig::regtest(1);
        let config2 = config1.clone();
        
        assert_eq!(config1.bitcoin_rpc_url, config2.bitcoin_rpc_url);
        assert_eq!(config1.relay_id, config2.relay_id);
        assert_eq!(config1.strfry_port, config2.strfry_port);
    }

    #[test]
    fn test_with_bitcoin_auth() {
        let config = RelayConfig::regtest(1)
            .with_bitcoin_auth("custom_user".to_string(), "custom_pass".to_string());
        
        assert_eq!(config.bitcoin_rpc_username, "custom_user");
        assert_eq!(config.bitcoin_rpc_password, "custom_pass");
        
        // Other fields should remain unchanged
        assert_eq!(config.relay_id, 1);
        assert_eq!(config.bitcoin_rpc_port, 18332);
    }

    #[test]
    fn test_with_validation_config() {
        let mut validation_config = ValidationConfig::default();
        validation_config.enable_validation = false;
        validation_config.cache_ttl_seconds = 300;
        
        let config = RelayConfig::regtest(1)
            .with_validation_config(validation_config.clone());
        
        assert_eq!(config.validation_config.enable_validation, false);
        assert_eq!(config.validation_config.cache_ttl_seconds, 300);
    }

    #[test]
    fn test_with_mempool_poll_interval() {
        let config = RelayConfig::regtest(1)
            .with_mempool_poll_interval(5);
        
        assert_eq!(config.mempool_poll_interval_secs, 5);
        
        // Other fields should remain unchanged
        assert_eq!(config.relay_id, 1);
        assert_eq!(config.bitcoin_rpc_port, 18332);
    }

    #[test]
    fn test_builder_pattern_chain() {
        let config = RelayConfig::testnet4(2)
            .with_bitcoin_auth("testuser".to_string(), "testpass".to_string())
            .with_mempool_poll_interval(10);
        
        // Check all configured values
        assert_eq!(config.relay_id, 2);
        assert_eq!(config.bitcoin_rpc_port, 48350);
        assert_eq!(config.bitcoin_rpc_username, "testuser");
        assert_eq!(config.bitcoin_rpc_password, "testpass");
        assert_eq!(config.mempool_poll_interval_secs, 10);
        assert_eq!(config.strfry_port, 7778);
    }

    #[test]
    fn test_validation_config_integration() {
        let config = RelayConfig::regtest(1);
        
        // ValidationConfig should have sensible defaults
        assert_eq!(config.validation_config.enable_validation, true);
        assert_eq!(config.validation_config.enable_precheck, true);
        assert!(config.validation_config.validation_timeout_ms > 0);
        assert!(config.validation_config.cache_ttl_seconds > 0);
        assert!(config.validation_config.cache_size > 0);
    }

    #[test]
    fn test_config_debug_format() {
        let config = RelayConfig::regtest(1);
        let debug_str = format!("{:?}", config);
        
        // Should contain key configuration values
        assert!(debug_str.contains("relay_id: 1"));
        assert!(debug_str.contains("18332"));
        assert!(debug_str.contains("7777"));
    }
}