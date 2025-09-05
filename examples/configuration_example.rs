use bitcoin_nostr_relay::{BitcoinNostrRelay, RelayConfig, Network, network_config, Result};
use std::net::SocketAddr;

fn main() -> Result<()> {
    println!("Bitcoin-Nostr Relay Configuration Examples");
    println!("==========================================\n");

    // METHOD 1: Convenience method (recommended for common patterns)
    println!("🎯 METHOD 1: Convenience method (recommended for common patterns):");
    let config_convenience = RelayConfig::for_network(Network::Regtest, 1)
        .with_auth("custom_user".to_string(), "custom_password".to_string())
        .with_mempool_poll_interval_secs(5);
    let _relay_convenience = BitcoinNostrRelay::new(config_convenience)?;
    println!("  RelayConfig::for_network(Network::Regtest, 1) - follows mature Rust patterns\n");
    
    // METHOD 2: Functional style (alternative)
    println!("⚡ METHOD 2: Functional style (alternative):");
    let config_functional = network_config(Network::Testnet4, 2)
        .with_auth("testnet_user".to_string(), "testnet_password".to_string())
        .with_mempool_poll_interval_secs(3);
    let _relay_functional = BitcoinNostrRelay::new(config_functional)?;
    println!("  network_config(Network::Testnet4, 2) - functional style\n");

    // METHOD 3: Explicit configuration (recommended for custom deployments)
    println!("🔧 METHOD 3: Explicit configuration (recommended for custom deployments):");
    let config_explicit = RelayConfig::new(
        "http://my-bitcoin-node:8332".to_string(),    // Bitcoin RPC URL
        "wss://my-nostr-relay.com".to_string(),       // Nostr relay URL
        "production-relay-1".to_string(),             // Custom relay ID
        "0.0.0.0:9001".parse::<SocketAddr>()?,       // WebSocket listen address
    )?
    .with_auth("bitcoind_user".to_string(), "secure_password".to_string())
    .with_mempool_poll_interval_secs(2);

    let _relay_explicit = BitcoinNostrRelay::new(config_explicit)?;
    println!("  RelayConfig::new(...) - full control over all URLs and addresses\n");

    // METHOD 4: Custom validation configuration
    println!("⚙️  METHOD 4: Custom validation configuration:");
    let mut validation_config = bitcoin_nostr_relay::ValidationConfig::default();
    validation_config.enable_validation = false; // Disable validation for testing
    validation_config.cache_size = 5000; // Larger cache
    
    let config_custom_validation = RelayConfig::for_network(Network::Regtest, 1)
        .with_auth("dev_user".to_string(), "dev_password".to_string())
        .with_validation(validation_config);
    
    let _relay_custom = BitcoinNostrRelay::new(config_custom_validation)?;
    println!("  Custom validation settings with builder pattern\n");

    // Benefits of the configuration approach
    println!("✅ Benefits of this configuration architecture:");
    println!("  🏗️  Follows mature Rust patterns (like tokio::Runtime::Builder)");
    println!("  🌐 No hardcoded port mappings or network assumptions");
    println!("  🔗 Connect to any Bitcoin RPC and Nostr relay URLs");
    println!("  🏷️  Supports custom relay IDs (strings, not just numbers)");
    println!("  📍 Explicit WebSocket listen addresses");
    println!("  🔄 Can run multiple relays with any configuration");
    println!("  🧪 Easy to test with different configurations");
    println!("  🚀 Deployment-agnostic library design");
    println!("  🎨 Multiple API styles: method-based and functional");
    println!("  🧹 Clean API without legacy complexity");

    Ok(())
}