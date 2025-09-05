pub mod bitcoin_rpc;
pub mod validation;
pub mod nostr;
pub mod relay;
pub mod networks;
pub mod error;

// Re-export core types for easy access
pub use bitcoin_rpc::BitcoinRpcClient;
pub use validation::{TransactionValidator, ValidationConfig};
pub use nostr::NostrClient;
pub use relay::{RelayServer, RelayConfig};
pub use networks::{Network, network_config};
pub use error::{RelayError, ConfigError, BitcoinRpcError, NostrError, ValidationError, NetworkError};

/// Library result type using our custom error
pub type Result<T, E = RelayError> = std::result::Result<T, E>;

/// High-level API for Bitcoin-over-Nostr relay functionality
pub struct BitcoinNostrRelay {
    bitcoin_client: BitcoinRpcClient,
    nostr_client: Option<NostrClient>,
    validator: TransactionValidator,
    config: RelayConfig,
}

impl BitcoinNostrRelay {
    /// Create a new BitcoinNostrRelay instance with the given configuration
    pub fn new(config: RelayConfig) -> Result<Self> {
        let bitcoin_client = BitcoinRpcClient::new(
            config.bitcoin_rpc_url.clone(),
            config.bitcoin_rpc_auth.username.clone(),
            config.bitcoin_rpc_auth.password.clone(),
        );
        
        // Extract port from Bitcoin RPC URL for validator
        let bitcoin_port = if let Ok(url) = url::Url::parse(&config.bitcoin_rpc_url) {
            url.port().unwrap_or(18332)
        } else {
            18332
        };
        
        let validator = TransactionValidator::new(
            config.validation_config.clone(),
            bitcoin_port,
        );
        
        Ok(Self {
            bitcoin_client,
            nostr_client: None,
            validator,
            config,
        })
    }
    
    /// Connect to the Nostr relay
    pub async fn connect_nostr(&mut self, ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>) -> Result<()> {
        self.nostr_client = Some(NostrClient::new(ws_stream));
        Ok(())
    }
    
    /// Start the relay server (monitors mempool and relays transactions)
    pub async fn start(&mut self) -> Result<()> {
        let relay_server = RelayServer::new(
            self.bitcoin_client.clone(),
            self.nostr_client.take(),
            self.validator.clone(),
            self.config.clone(),
        )?;
        
        relay_server.run().await
    }
    
    /// Broadcast a transaction to the Nostr network
    pub async fn broadcast_transaction(&self, tx_hex: &str, block_hash: &str) -> Result<()> {
        if let Some(nostr_client) = &self.nostr_client {
            nostr_client.send_tx_event(tx_hex, block_hash).await.map_err(RelayError::from)
        } else {
            Err(NostrError::Disconnected.into())
        }
    }
    
    /// Validate a transaction using the configured validator
    pub async fn validate_transaction(&self, tx_hex: &str) -> Result<(), ValidationError> {
        self.validator.validate(tx_hex).await
    }
    
    /// Get the relay configuration
    pub fn config(&self) -> &RelayConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::RelayConfig;
    
    #[test]
    fn test_bitcoin_nostr_relay_creation() {
        let config = RelayConfig::for_network(Network::Regtest, 1);
        let relay = BitcoinNostrRelay::new(config);
        
        assert!(relay.is_ok());
        let relay = relay.unwrap();
        
        // Should be created without Nostr client initially
        assert!(relay.nostr_client.is_none());
    }
    
    #[test]
    fn test_bitcoin_nostr_relay_with_different_configs() {
        // Test regtest config
        let regtest_config = RelayConfig::for_network(Network::Regtest, 1);
        let regtest_relay = BitcoinNostrRelay::new(regtest_config);
        assert!(regtest_relay.is_ok());
        
        // Test testnet4 config
        let testnet_config = RelayConfig::for_network(Network::Testnet4, 2);
        let testnet_relay = BitcoinNostrRelay::new(testnet_config);
        assert!(testnet_relay.is_ok());
        
        // Test custom config
        let custom_config = RelayConfig::new(
            "http://127.0.0.1:19000".to_string(),
            "ws://127.0.0.1:8000".to_string(),
            "3".to_string(),
            "127.0.0.1:7781".parse().unwrap(),
        ).unwrap();
        let custom_relay = BitcoinNostrRelay::new(custom_config);
        assert!(custom_relay.is_ok());
    }
    
    #[test]
    fn test_bitcoin_nostr_relay_with_validation_config() {
        let mut validation_config = ValidationConfig::default();
        validation_config.enable_validation = false;
        validation_config.cache_size = 500;
        
        let config = RelayConfig::for_network(Network::Regtest, 1)
            .with_validation(validation_config.clone());
            
        let relay = BitcoinNostrRelay::new(config);
        assert!(relay.is_ok());
        
        let relay = relay.unwrap();
        assert_eq!(relay.validator.config().enable_validation, false);
        assert_eq!(relay.validator.config().cache_size, 500);
    }
    
    #[tokio::test]
    async fn test_validate_transaction_with_disabled_validation() {
        let mut validation_config = ValidationConfig::default();
        validation_config.enable_validation = false;
        
        let config = RelayConfig::for_network(Network::Regtest, 1)
            .with_validation(validation_config);
            
        let relay = BitcoinNostrRelay::new(config).unwrap();
        
        // Should pass validation when disabled, even with invalid hex
        let result = relay.validate_transaction("invalid_hex").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_validate_transaction_with_empty_input() {
        let config = RelayConfig::for_network(Network::Regtest, 1);
        let relay = BitcoinNostrRelay::new(config).unwrap();
        
        // Should fail validation with empty transaction
        let result = relay.validate_transaction("").await;
        assert!(result.is_err());
        
        // Could be EmptyTransaction or InvalidStructure depending on validation order
        match result {
            Err(ValidationError::EmptyTransaction) => {
                // Expected error type
            }
            Err(ValidationError::InvalidStructure) => {
                // Also acceptable as TXID extraction fails first
            }
            _ => panic!("Expected EmptyTransaction or InvalidStructure error, got: {:?}", result)
        }
    }
    
    #[tokio::test] 
    async fn test_validate_transaction_with_invalid_hex() {
        let config = RelayConfig::for_network(Network::Regtest, 1);
        let relay = BitcoinNostrRelay::new(config).unwrap();
        
        // Should fail validation with invalid hex
        let result = relay.validate_transaction("not_hex_characters").await;
        assert!(result.is_err());
        
        if let Err(ValidationError::InvalidHex) = result {
            // Expected error type
        } else {
            panic!("Expected InvalidHex error, got: {:?}", result);
        }
    }
    
    #[tokio::test]
    async fn test_validate_transaction_with_invalid_size() {
        let config = RelayConfig::for_network(Network::Regtest, 1);
        let relay = BitcoinNostrRelay::new(config).unwrap();
        
        // Should fail validation with transaction too small (less than 60 bytes)
        let small_tx = "a".repeat(118); // 59 bytes
        let result = relay.validate_transaction(&small_tx).await;
        assert!(result.is_err());
        
        // Could be InvalidSize (from precheck) or InvalidStructure (from TXID extraction)
        match result {
            Err(ValidationError::InvalidSize { size: 59 }) => {
                // Expected error type from precheck
            }
            Err(ValidationError::InvalidStructure) => {
                // Also acceptable as TXID extraction fails first
            }
            _ => panic!("Expected InvalidSize{{size: 59}} or InvalidStructure error, got: {:?}", result)
        }
    }
    
    #[tokio::test]
    async fn test_broadcast_transaction_without_nostr_client() {
        let config = RelayConfig::for_network(Network::Regtest, 1);
        let relay = BitcoinNostrRelay::new(config).unwrap();
        
        // Should fail to broadcast without Nostr client
        let result = relay.broadcast_transaction("deadbeef", "block_hash").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Nostr relay disconnected"));
    }
    
    #[test]
    fn test_bitcoin_nostr_relay_config_integration() {
        let config = RelayConfig::for_network(Network::Regtest, 1)
            .with_auth("custom_user".to_string(), "custom_pass".to_string())
            .with_mempool_poll_interval_secs(5);
            
        let relay = BitcoinNostrRelay::new(config).unwrap();
        
        // Config should be properly integrated
        assert_eq!(relay.config.bitcoin_rpc_auth.username, "custom_user");
        assert_eq!(relay.config.bitcoin_rpc_auth.password, "custom_pass");
        assert_eq!(relay.config.mempool_poll_interval.as_secs(), 5);
    }
    
    // Integration test that would require a real WebSocket connection
    #[tokio::test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    async fn test_connect_nostr_integration() {
        // This test would require setting up a real Nostr relay connection
        // For now, we'll skip it in regular test runs
        
        // In a real integration test, you would:
        // 1. Set up a test Nostr relay (like strfry in test mode)
        // 2. Connect to it via WebSocket
        // 3. Create BitcoinNostrRelay and connect
        // 4. Test broadcasting transactions
        
        // Example structure:
        // let config = RelayConfig::regtest(1);
        // let mut relay = BitcoinNostrRelay::new(config).unwrap();
        // let url = "ws://localhost:7777";
        // let (ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        // relay.connect_nostr(ws_stream).await.unwrap();
        // let result = relay.broadcast_transaction("deadbeef", "block_hash").await;
        // assert!(result.is_ok());
    }
    
    // Integration test that would require a running Bitcoin node
    #[tokio::test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    async fn test_start_relay_integration() {
        // This test would require a full integration setup
        // For now, we'll skip it in regular test runs
        
        // In a real integration test, you would:
        // 1. Set up test Bitcoin node
        // 2. Set up test Nostr relay
        // 3. Create BitcoinNostrRelay with test config
        // 4. Start the relay server
        // 5. Test transaction flow
        
        // Example structure:
        // let config = RelayConfig::regtest(1);
        // let mut relay = BitcoinNostrRelay::new(config).unwrap();
        // // Connect WebSocket, then start
        // let result = relay.start().await;
        // assert!(result.is_ok());
    }
}