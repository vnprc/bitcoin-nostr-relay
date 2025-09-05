# bitcoin-nostr-relay

[![Crates.io](https://img.shields.io/crates/v/bitcoin-nostr-relay.svg)](https://crates.io/crates/bitcoin-nostr-relay)
[![Documentation](https://docs.rs/bitcoin-nostr-relay/badge.svg)](https://docs.rs/bitcoin-nostr-relay)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tests](https://img.shields.io/badge/tests-52%20passing-brightgreen.svg)](#testing)

A Rust library for relaying Bitcoin transactions over the Nostr protocol, enabling censorship-resistant transaction propagation networks.

> **🚀 Modern Architecture**: Deployment-agnostic configuration following mature Rust patterns. Connect to any Bitcoin RPC and Nostr relay URLs with no hardcoded assumptions.

## Features

- **🔗 Bitcoin RPC Integration**: Connect to any Bitcoin Core node via URL
- **📡 Nostr Protocol Support**: Connect to any Nostr relay via WebSocket URL  
- **✅ Transaction Validation**: Configurable validation with caching and anti-spam protection
- **🌐 Deployment Agnostic**: No hardcoded ports - works with any network configuration
- **🎯 Mature Rust Patterns**: Builder patterns following tokio/reqwest conventions
- **🔧 Flexible Configuration**: Multiple API styles (method-based, functional, explicit)
- **📋 Comprehensive Testing**: 52 tests covering all functionality with integration scenarios

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
bitcoin-nostr-relay = "0.1"  # Use latest version for new configuration API
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use bitcoin_nostr_relay::*;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> { // Using library Result type
    // Method-based convenience (recommended for common patterns)
    let config = RelayConfig::for_network(Network::Regtest, 1)
        .with_auth("user".to_string(), "password".to_string())
        .with_mempool_poll_interval_secs(5);
    
    // Explicit configuration with validation (recommended for custom deployments)
    let config = RelayConfig::new(
        "http://127.0.0.1:18332",    // Bitcoin RPC URL
        "ws://127.0.0.1:7777",       // Nostr relay URL  
        "my-relay",                  // Relay ID
        "127.0.0.1:7779".parse()?,   // WebSocket listen address
    )?  // Validates URLs at construction time
    .with_auth("user".to_string(), "password".to_string());
    
    // Create the relay instance
    let mut relay = BitcoinNostrRelay::new(config)?;
    
    // Validate a transaction with structured error handling
    let tx_hex = "deadbeef..."; // Your transaction hex
    match relay.validate_transaction(tx_hex).await {
        Ok(()) => println!("Transaction is valid"),
        Err(ValidationError::InvalidHex) => println!("Invalid hex format"),
        Err(e) => println!("Validation failed: {}", e),
    }
    
    // Connect to Nostr relay and start broadcasting
    // let (ws_stream, _) = tokio_tungstenite::connect_async("ws://localhost:7777").await?;
    // relay.connect_nostr(ws_stream).await?;
    // relay.start().await?;
    
    Ok(())
}
```

### Configuration

The library provides multiple configuration approaches following mature Rust patterns:

```rust
use bitcoin_nostr_relay::*;
use std::net::SocketAddr;

// Method-based convenience (like tokio::Runtime::Builder)
let config = RelayConfig::for_network(Network::Regtest, 1)
    .with_auth("user".to_string(), "pass".to_string())
    .with_mempool_poll_interval_secs(5);

// Functional convenience (alternative style)
let config = network_config(Network::Testnet4, 1)
    .with_auth("user".to_string(), "pass".to_string());

// Explicit configuration with validation (full control)
let config = RelayConfig::new(
    "http://your-bitcoin-node:8332",
    "wss://your-nostr-relay.com", 
    "production-relay-1",
    "0.0.0.0:9001".parse()?,
)?  // Validates URLs and parameters at construction time
.with_auth("bitcoin_user".to_string(), "secure_password".to_string());

// Custom validation settings
let mut validation_config = ValidationConfig::default();
validation_config.enable_validation = false; // Disable validation
validation_config.cache_size = 2000; // Larger cache

let config = RelayConfig::for_network(Network::Regtest, 1)
    .with_validation(validation_config);
```

## API Reference

### Core Components

- **`BitcoinNostrRelay`**: High-level API for relay functionality
- **`RelayConfig`**: Configuration builder with chain-specific presets
- **`BitcoinRpcClient`**: Bitcoin Core RPC client
- **`NostrClient`**: Nostr protocol client for WebSocket communication
- **`TransactionValidator`**: Transaction validation with caching
- **`RelayServer`**: Low-level relay server implementation

### Key Methods

#### BitcoinNostrRelay

```rust
impl BitcoinNostrRelay {
    pub fn new(config: RelayConfig) -> Result<Self>;
    pub async fn connect_nostr(&mut self, ws_stream: WebSocketStream) -> Result<()>;
    pub async fn start(&mut self) -> Result<()>;
    pub async fn broadcast_transaction(&self, tx_hex: &str, block_hash: &str) -> Result<()>;
    pub async fn validate_transaction(&self, tx_hex: &str) -> Result<(), ValidationError>;
    pub fn config(&self) -> &RelayConfig;
}
```

#### RelayConfig

```rust
impl RelayConfig {
    // Primary constructor (deployment-agnostic)
    pub fn new(
        bitcoin_rpc_url: String,
        strfry_url: String, 
        relay_id: String,
        websocket_listen_addr: SocketAddr,
    ) -> Self;
    
    // Convenience constructors following mature Rust patterns
    pub fn for_network(network: Network, relay_id: u16) -> Self;
    
    // Builder methods
    pub fn with_auth(self, username: String, password: String) -> Self;
    pub fn with_validation(self, config: ValidationConfig) -> Self;
    pub fn with_mempool_poll_interval(self, interval: Duration) -> Self;
    pub fn with_mempool_poll_interval_secs(self, seconds: u64) -> Self;
}

// Standalone convenience function
pub fn network_config(network: Network, relay_id: u16) -> RelayConfig;

pub enum Network {
    Regtest,
    Testnet4,
}
```

## Architecture

The library implements a **deployment-agnostic** Bitcoin-over-Nostr relay network:

### Network Flow
1. **Bitcoin nodes** provide transaction data via RPC (any URL)
2. **Relay servers** monitor mempools and broadcast transactions  
3. **Nostr relays** facilitate peer-to-peer transaction sharing (any WebSocket URL)
4. **Validation layer** prevents spam and validates transactions
5. **High-level API** simplifies integration with flexible configuration

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│ Bitcoin     │◄──►│ tx-relay     │◄──►│ Nostr       │
│ Node        │    │ Library      │    │ Relay       │
│ (Any URL)   │    │              │    │ (Any WSS)   │
└─────────────┘    └──────────────┘    └─────────────┘
```

### Key Architectural Benefits

✅ **Deployment Agnostic**: No hardcoded ports or network assumptions  
✅ **URL-Based Configuration**: Connect to any Bitcoin RPC and Nostr relay  
✅ **Flexible Relay IDs**: String-based IDs support any naming scheme  
✅ **Multiple API Styles**: Method-based, functional, and explicit patterns  
✅ **Mature Rust Patterns**: Follows conventions from tokio, reqwest, clap  
✅ **Structured Error Handling**: Custom error types with detailed context  
✅ **Fail-Fast Validation**: URLs and parameters validated at construction time  
✅ **Clean API**: Simple, focused API without legacy complexity  
✅ **Comprehensive Testing**: 53 tests covering all configuration scenarios

## Error Handling

The library provides comprehensive, structured error types for better debugging and error handling:

```rust
use bitcoin_nostr_relay::{Result, RelayError, ValidationError, ConfigError};

// Library-wide Result type
fn create_relay() -> Result<BitcoinNostrRelay> {
    let config = RelayConfig::new(
        "http://127.0.0.1:18332",
        "ws://127.0.0.1:7777", 
        "my-relay",
        "127.0.0.1:7779".parse()?,
    )?; // Validates URLs at construction time
    
    BitcoinNostrRelay::new(config)
}

// Structured error handling with detailed context
match relay.validate_transaction(tx_hex).await {
    Ok(()) => println!("Transaction valid"),
    Err(ValidationError::EmptyTransaction) => println!("Empty transaction"),
    Err(ValidationError::InvalidHex) => println!("Invalid hex format"),
    Err(ValidationError::InvalidSize { size }) => println!("Invalid size: {} bytes", size),
    Err(ValidationError::RecentlyProcessed { txid }) => println!("Recently processed: {}", txid),
    Err(ValidationError::BitcoinCoreRejection { reason }) => println!("Rejected: {}", reason),
    Err(e) => println!("Other error: {}", e),
}

// Configuration validation errors
match RelayConfig::new("invalid-url", "ws://localhost:7777", "relay", addr) {
    Ok(config) => println!("Configuration valid"),
    Err(ConfigError::InvalidUrl { url }) => println!("Invalid URL: {}", url),
    Err(ConfigError::InvalidParameter { param }) => println!("Invalid parameter: {}", param),
    Err(e) => println!("Configuration error: {}", e),
}
```

### Error Types

The library provides these structured error types:

- **`RelayError`**: Top-level error type for all library operations
- **`ConfigError`**: Configuration validation errors
- **`ValidationError`**: Transaction validation errors  
- **`BitcoinRpcError`**: Bitcoin RPC communication errors
- **`NostrError`**: Nostr protocol errors
- **`NetworkError`**: Network and connection errors

All errors implement `std::error::Error` and provide detailed context for debugging.

## Testing

Run the test suite:

```bash
# All tests (recommended)
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# Include integration tests requiring external services
cargo test -- --include-ignored
```

The library includes:
- **🧪 52 total tests** with comprehensive coverage
- **45 unit tests** covering all core functionality and configuration patterns
- **7 integration tests** for end-to-end scenarios 
- **Ignored tests** for scenarios requiring external services (Bitcoin Core, Nostr relays)
- **Configuration tests** verifying all configuration patterns

## Examples

### Quick Start Examples

Run the configuration example to see all patterns:

```bash
cargo run --example configuration_example
```

### Complete Integration Examples

See the [playground repository](https://github.com/vnprc/tx-relay-playground) for complete examples including:
- Multi-relay setup with nix
- Bitcoin regtest and testnet4 configurations  
- End-to-end transaction relay demonstrations
- Production deployment patterns

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Safety and Security

⚠️ **This is experimental software**. Do not use in production without thorough testing and security review.

- Transaction validation is configurable but not foolproof
- The library trusts the configured Bitcoin node
- Nostr relays can potentially censor or manipulate events
- Always validate critical transactions through multiple sources