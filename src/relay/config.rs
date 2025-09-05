use crate::validation::ValidationConfig;
use std::net::SocketAddr;
use std::time::Duration;

/// Authentication credentials for Bitcoin RPC
#[derive(Debug, Clone)]
pub struct RpcAuth {
    pub username: String,
    pub password: String,
}

/// Configuration for the Bitcoin-Nostr relay server
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Bitcoin RPC URL (e.g., "http://127.0.0.1:18332")
    pub bitcoin_rpc_url: String,
    
    /// Bitcoin RPC authentication credentials
    pub bitcoin_rpc_auth: RpcAuth,
    
    /// Strfry Nostr relay URL (e.g., "ws://127.0.0.1:7777")
    pub strfry_url: String,
    
    /// Relay identifier (unique for each relay instance)
    pub relay_id: String,
    
    /// WebSocket server listen address
    pub websocket_listen_addr: SocketAddr,
    
    /// Configuration for transaction validation
    pub validation_config: ValidationConfig,
    
    /// Mempool polling interval
    pub mempool_poll_interval: Duration,
    
    /// Maximum number of concurrent client connections
    pub max_client_connections: usize,
    
    /// WebSocket buffer size for client connections
    pub websocket_buffer_size: usize,
}

impl RelayConfig {
    /// Create a new RelayConfig with the provided URLs and addresses
    pub fn new(
        bitcoin_rpc_url: String,
        strfry_url: String,
        relay_id: String,
        websocket_listen_addr: SocketAddr,
    ) -> Self {
        Self {
            bitcoin_rpc_url,
            bitcoin_rpc_auth: RpcAuth {
                username: "user".to_string(),
                password: "password".to_string(),
            },
            strfry_url,
            relay_id,
            websocket_listen_addr,
            validation_config: ValidationConfig::default(),
            mempool_poll_interval: Duration::from_secs(2),
            max_client_connections: 1000,
            websocket_buffer_size: 100,
        }
    }
    
    
    /// Set custom Bitcoin RPC credentials
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.bitcoin_rpc_auth = RpcAuth { username, password };
        self
    }
    
    /// Set custom validation configuration
    pub fn with_validation(mut self, config: ValidationConfig) -> Self {
        self.validation_config = config;
        self
    }
    
    /// Set custom mempool polling interval
    pub fn with_mempool_poll_interval(mut self, interval: Duration) -> Self {
        self.mempool_poll_interval = interval;
        self
    }
    
    /// Backward compatibility: Set mempool polling interval from seconds
    pub fn with_mempool_poll_interval_secs(mut self, seconds: u64) -> Self {
        self.mempool_poll_interval = Duration::from_secs(seconds);
        self
    }
    
    /// Create a configuration for common network patterns (recommended convenience method)
    /// 
    /// This provides the same functionality as the standalone `network_config` function
    /// but follows the pattern used by mature Rust libraries like tokio, reqwest, etc.
    pub fn for_network(network: crate::networks::Network, relay_id: u16) -> Self {
        crate::networks::network_config(network, relay_id)
    }
    
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self::for_network(crate::networks::Network::Regtest, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_config_new() {
        let config = RelayConfig::new(
            "http://127.0.0.1:18332".to_string(),
            "ws://127.0.0.1:7777".to_string(),
            "test-relay".to_string(),
            "127.0.0.1:7779".parse().unwrap(),
        );
        
        assert_eq!(config.bitcoin_rpc_url, "http://127.0.0.1:18332");
        assert_eq!(config.bitcoin_rpc_auth.username, "user");
        assert_eq!(config.bitcoin_rpc_auth.password, "password");
        assert_eq!(config.strfry_url, "ws://127.0.0.1:7777");
        assert_eq!(config.relay_id, "test-relay");
        assert_eq!(config.websocket_listen_addr, "127.0.0.1:7779".parse::<SocketAddr>().unwrap());
        assert_eq!(config.mempool_poll_interval, Duration::from_secs(2));
        assert_eq!(config.max_client_connections, 1000);
        assert_eq!(config.websocket_buffer_size, 100);
    }

    #[test]
    fn test_relay_config_for_network_regtest() {
        let config1 = RelayConfig::for_network(crate::networks::Network::Regtest, 1);
        assert_eq!(config1.bitcoin_rpc_url, "http://127.0.0.1:18332");
        assert_eq!(config1.strfry_url, "ws://127.0.0.1:7777");
        assert_eq!(config1.relay_id, "1");
        assert_eq!(config1.websocket_listen_addr, "127.0.0.1:7779".parse::<SocketAddr>().unwrap());
        
        let config2 = RelayConfig::for_network(crate::networks::Network::Regtest, 2);
        assert_eq!(config2.bitcoin_rpc_url, "http://127.0.0.1:18444");
        assert_eq!(config2.strfry_url, "ws://127.0.0.1:7778");
        assert_eq!(config2.relay_id, "2");
        assert_eq!(config2.websocket_listen_addr, "127.0.0.1:7780".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_relay_config_for_network_testnet4() {
        let config1 = RelayConfig::for_network(crate::networks::Network::Testnet4, 1);
        assert_eq!(config1.bitcoin_rpc_url, "http://127.0.0.1:48330");
        assert_eq!(config1.strfry_url, "ws://127.0.0.1:7777");
        assert_eq!(config1.relay_id, "1");
        
        let config2 = RelayConfig::for_network(crate::networks::Network::Testnet4, 2);
        assert_eq!(config2.bitcoin_rpc_url, "http://127.0.0.1:48350");
        assert_eq!(config2.strfry_url, "ws://127.0.0.1:7778");
        assert_eq!(config2.relay_id, "2");
    }

    #[test]
    fn test_relay_config_default() {
        let config = RelayConfig::default();
        
        // Default should be regtest relay 1
        assert_eq!(config.relay_id, "1");
        assert_eq!(config.bitcoin_rpc_url, "http://127.0.0.1:18332");
        assert_eq!(config.strfry_url, "ws://127.0.0.1:7777");
    }

    #[test]
    fn test_relay_config_clone() {
        let config1 = RelayConfig::for_network(crate::networks::Network::Regtest, 1);
        let config2 = config1.clone();
        
        assert_eq!(config1.bitcoin_rpc_url, config2.bitcoin_rpc_url);
        assert_eq!(config1.relay_id, config2.relay_id);
        assert_eq!(config1.strfry_url, config2.strfry_url);
    }

    #[test]
    fn test_with_auth() {
        let config = RelayConfig::for_network(crate::networks::Network::Regtest, 1)
            .with_auth("custom_user".to_string(), "custom_pass".to_string());
        
        assert_eq!(config.bitcoin_rpc_auth.username, "custom_user");
        assert_eq!(config.bitcoin_rpc_auth.password, "custom_pass");
        
        // Other fields should remain unchanged
        assert_eq!(config.relay_id, "1");
        assert_eq!(config.bitcoin_rpc_url, "http://127.0.0.1:18332");
    }

    #[test]
    fn test_with_validation_config() {
        let mut validation_config = ValidationConfig::default();
        validation_config.enable_validation = false;
        validation_config.cache_ttl_seconds = 300;
        
        let config = RelayConfig::for_network(crate::networks::Network::Regtest, 1)
            .with_validation(validation_config.clone());
        
        assert_eq!(config.validation_config.enable_validation, false);
        assert_eq!(config.validation_config.cache_ttl_seconds, 300);
    }

    #[test]
    fn test_with_mempool_poll_interval() {
        let config = RelayConfig::for_network(crate::networks::Network::Regtest, 1)
            .with_mempool_poll_interval(Duration::from_secs(5));
        
        assert_eq!(config.mempool_poll_interval.as_secs(), 5);
        
        // Other fields should remain unchanged
        assert_eq!(config.relay_id, "1");
        assert_eq!(config.bitcoin_rpc_url, "http://127.0.0.1:18332");
    }

    #[test]
    fn test_builder_pattern_chain() {
        let config = RelayConfig::for_network(crate::networks::Network::Testnet4, 2)
            .with_auth("testuser".to_string(), "testpass".to_string())
            .with_mempool_poll_interval_secs(10);
        
        // Check all configured values
        assert_eq!(config.relay_id, "2");
        assert_eq!(config.bitcoin_rpc_url, "http://127.0.0.1:48350");
        assert_eq!(config.bitcoin_rpc_auth.username, "testuser");
        assert_eq!(config.bitcoin_rpc_auth.password, "testpass");
        assert_eq!(config.mempool_poll_interval.as_secs(), 10);
        assert_eq!(config.strfry_url, "ws://127.0.0.1:7778");
    }

    #[test]
    fn test_validation_config_integration() {
        let config = RelayConfig::for_network(crate::networks::Network::Regtest, 1);
        
        // ValidationConfig should have sensible defaults
        assert_eq!(config.validation_config.enable_validation, true);
        assert_eq!(config.validation_config.enable_precheck, true);
        assert!(config.validation_config.validation_timeout_ms > 0);
        assert!(config.validation_config.cache_ttl_seconds > 0);
        assert!(config.validation_config.cache_size > 0);
    }

    #[test]
    fn test_config_debug_format() {
        let config = RelayConfig::for_network(crate::networks::Network::Regtest, 1);
        let debug_str = format!("{:?}", config);
        
        // Should contain key configuration values
        assert!(debug_str.contains("relay_id: \"1\""));
        assert!(debug_str.contains("18332"));
        assert!(debug_str.contains("7777"));
    }

    #[test]
    fn test_for_network_convenience_method() {
        // Test the new convenience method that follows mature Rust patterns
        let config1 = RelayConfig::for_network(crate::networks::Network::Regtest, 1)
            .with_auth("user".to_string(), "pass".to_string())
            .with_mempool_poll_interval_secs(5);
            
        assert_eq!(config1.bitcoin_rpc_url, "http://127.0.0.1:18332");
        assert_eq!(config1.strfry_url, "ws://127.0.0.1:7777");
        assert_eq!(config1.relay_id, "1");
        assert_eq!(config1.bitcoin_rpc_auth.username, "user");
        assert_eq!(config1.bitcoin_rpc_auth.password, "pass");
        assert_eq!(config1.mempool_poll_interval.as_secs(), 5);
        
        // Test testnet4
        let config2 = RelayConfig::for_network(crate::networks::Network::Testnet4, 2);
        assert_eq!(config2.bitcoin_rpc_url, "http://127.0.0.1:48350");
        assert_eq!(config2.strfry_url, "ws://127.0.0.1:7778");
        assert_eq!(config2.relay_id, "2");
    }
}