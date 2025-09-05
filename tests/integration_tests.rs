use tx_relay::*;

#[test]
fn test_library_integration_basic() {
    // Test that all library components can work together
    let config = RelayConfig::regtest(1)
        .with_bitcoin_auth("user".to_string(), "password".to_string())
        .with_mempool_poll_interval(5);
    
    // Create BitcoinNostrRelay with the config
    let relay = BitcoinNostrRelay::new(config);
    assert!(relay.is_ok());
    
    // Test that components are properly configured
    let relay = relay.unwrap();
    assert_eq!(relay.config().relay_id, 1);
    assert_eq!(relay.config().bitcoin_rpc_username, "user");
    assert_eq!(relay.config().bitcoin_rpc_password, "password");
    assert_eq!(relay.config().mempool_poll_interval_secs, 5);
}

#[test]
fn test_library_integration_multiple_relays() {
    // Test creating multiple relay configurations
    let relay1 = BitcoinNostrRelay::new(RelayConfig::regtest(1)).unwrap();
    let relay2 = BitcoinNostrRelay::new(RelayConfig::regtest(2)).unwrap();
    
    // Each should have unique configuration
    assert_eq!(relay1.config().relay_id, 1);
    assert_eq!(relay1.config().bitcoin_rpc_port, 18332);
    assert_eq!(relay1.config().strfry_port, 7777);
    
    assert_eq!(relay2.config().relay_id, 2);
    assert_eq!(relay2.config().bitcoin_rpc_port, 18444);
    assert_eq!(relay2.config().strfry_port, 7778);
}

#[test]
fn test_library_integration_testnet4() {
    // Test testnet4 configuration integration
    let relay = BitcoinNostrRelay::new(RelayConfig::testnet4(1)).unwrap();
    
    assert_eq!(relay.config().relay_id, 1);
    assert_eq!(relay.config().bitcoin_rpc_port, 48330);
    assert_eq!(relay.config().strfry_port, 7777);
    assert_eq!(relay.config().bitcoin_rpc_url, "http://127.0.0.1:48330");
}

#[tokio::test]
async fn test_library_integration_validation_flow() {
    // Test the complete validation flow
    let config = RelayConfig::regtest(1);
    let relay = BitcoinNostrRelay::new(config).unwrap();
    
    // Test various validation scenarios
    assert!(relay.validate_transaction("").await.is_err()); // Empty
    assert!(relay.validate_transaction("not_hex").await.is_err()); // Invalid hex
    assert!(relay.validate_transaction(&"a".repeat(118)).await.is_err()); // Too small
    
    // Test with disabled validation
    let mut validation_config = ValidationConfig::default();
    validation_config.enable_validation = false;
    
    let config_disabled = RelayConfig::regtest(1)
        .with_validation_config(validation_config);
    let relay_disabled = BitcoinNostrRelay::new(config_disabled).unwrap();
    
    // Should pass even with invalid input when validation disabled
    assert!(relay_disabled.validate_transaction("invalid").await.is_ok());
}

#[test]
fn test_library_integration_builder_pattern() {
    // Test that the builder pattern works end-to-end
    let config = RelayConfig::new(20000, 99, 8888)
        .with_bitcoin_auth("custom_user".to_string(), "custom_password".to_string())
        .with_mempool_poll_interval(10);
    
    let relay = BitcoinNostrRelay::new(config).unwrap();
    
    // Verify all builder pattern values are applied
    assert_eq!(relay.config().bitcoin_rpc_port, 20000);
    assert_eq!(relay.config().relay_id, 99);
    assert_eq!(relay.config().strfry_port, 8888);
    assert_eq!(relay.config().bitcoin_rpc_username, "custom_user");
    assert_eq!(relay.config().bitcoin_rpc_password, "custom_password");
    assert_eq!(relay.config().mempool_poll_interval_secs, 10);
    assert_eq!(relay.config().bitcoin_rpc_url, "http://127.0.0.1:20000");
}

#[tokio::test]
async fn test_library_integration_error_handling() {
    // Test error handling across the library
    let config = RelayConfig::regtest(1);
    let relay = BitcoinNostrRelay::new(config).unwrap();
    
    // Test broadcast without Nostr client
    let broadcast_result = relay.broadcast_transaction("deadbeef", "block_hash").await;
    assert!(broadcast_result.is_err());
    assert!(broadcast_result.unwrap_err().to_string().contains("Nostr client not connected"));
    
    // Test validation errors
    let validation_result = relay.validate_transaction("").await;
    assert!(validation_result.is_err());
    
    // Test with different validation configurations
    let mut validation_config = ValidationConfig::default();
    validation_config.enable_precheck = false;
    
    let config_no_precheck = RelayConfig::regtest(1)
        .with_validation_config(validation_config);
    let relay_no_precheck = BitcoinNostrRelay::new(config_no_precheck).unwrap();
    
    // Should still fail but potentially with different error
    let result = relay_no_precheck.validate_transaction("").await;
    assert!(result.is_err());
}

// Integration test that would require running services
#[tokio::test]
#[ignore] // Use `cargo test -- --ignored` to run this test
async fn test_full_integration_with_services() {
    // This test would require:
    // 1. Running Bitcoin Core in regtest mode
    // 2. Running Strfry Nostr relay
    // 3. Complete end-to-end transaction flow testing
    
    // Example test structure (commented out since it requires external services):
    /*
    // Set up relay
    let config = RelayConfig::regtest(1);
    let mut relay = BitcoinNostrRelay::new(config).unwrap();
    
    // Connect to Nostr relay
    let url = "ws://localhost:7777";
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    relay.connect_nostr(ws_stream).await.unwrap();
    
    // Test transaction broadcasting
    let tx_hex = "valid_transaction_hex_here";
    let broadcast_result = relay.broadcast_transaction(tx_hex, "block_hash").await;
    assert!(broadcast_result.is_ok());
    
    // Test starting the relay server
    let start_result = relay.start().await;
    // This would run indefinitely, so we'd need to structure it differently
    */
}