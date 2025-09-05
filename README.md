# bitcoin-nostr-relay

[![Crates.io](https://img.shields.io/crates/v/bitcoin-nostr-relay.svg)](https://crates.io/crates/bitcoin-nostr-relay)
[![Documentation](https://docs.rs/bitcoin-nostr-relay/badge.svg)](https://docs.rs/bitcoin-nostr-relay)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust library for relaying Bitcoin transactions over the Nostr protocol, enabling censorship-resistant transaction propagation networks.

## Features

- **Bitcoin RPC Integration**: Connect to Bitcoin Core nodes for transaction monitoring and submission
- **Nostr Protocol Support**: Broadcast and receive Bitcoin transactions via Nostr relays
- **Transaction Validation**: Configurable validation with caching and anti-spam protection
- **Multi-Chain Support**: Built-in configurations for regtest and testnet4
- **High-Level API**: Simple interface for creating Bitcoin-over-Nostr relay networks
- **Comprehensive Testing**: Full unit and integration test coverage

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
bitcoin-nostr-relay = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use bitcoin_nostr_relay::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a regtest configuration
    let config = RelayConfig::regtest(1)
        .with_bitcoin_auth("user".to_string(), "password".to_string())
        .with_mempool_poll_interval(5);
    
    // Create the relay instance
    let mut relay = BitcoinNostrRelay::new(config)?;
    
    // Validate a transaction
    let tx_hex = "deadbeef..."; // Your transaction hex
    relay.validate_transaction(tx_hex).await?;
    
    // Connect to Nostr relay and start broadcasting
    // let (ws_stream, _) = tokio_tungstenite::connect_async("ws://localhost:7777").await?;
    // relay.connect_nostr(ws_stream).await?;
    // relay.start().await?;
    
    Ok(())
}
```

### Configuration

The library supports different Bitcoin networks and custom configurations:

```rust
use bitcoin_nostr_relay::*;

// Regtest configuration (default ports: Bitcoin 18332, Strfry 7777)
let regtest_config = RelayConfig::regtest(1);

// Testnet4 configuration (default ports: Bitcoin 48330, Strfry 7777)
let testnet_config = RelayConfig::testnet4(1);

// Custom configuration
let custom_config = RelayConfig::new(20000, 1, 8000)
    .with_bitcoin_auth("custom_user".to_string(), "custom_password".to_string())
    .with_mempool_poll_interval(10);

// Custom validation settings
let mut validation_config = ValidationConfig::default();
validation_config.enable_validation = false; // Disable validation
validation_config.cache_size = 2000; // Larger cache

let config = RelayConfig::regtest(1)
    .with_validation_config(validation_config);
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
    pub fn new(bitcoin_rpc_port: u16, relay_id: u16, strfry_port: u16) -> Self;
    pub fn regtest(relay_id: u16) -> Self;
    pub fn testnet4(relay_id: u16) -> Self;
    pub fn with_bitcoin_auth(self, username: String, password: String) -> Self;
    pub fn with_validation_config(self, config: ValidationConfig) -> Self;
    pub fn with_mempool_poll_interval(self, seconds: u64) -> Self;
}
```

## Architecture

The library implements a Bitcoin-over-Nostr relay network where:

1. **Bitcoin nodes** provide transaction data via RPC
2. **Relay servers** monitor mempools and broadcast transactions
3. **Nostr relays** facilitate peer-to-peer transaction sharing
4. **Validation layer** prevents spam and validates transactions
5. **High-level API** simplifies integration

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│ Bitcoin     │◄──►│ tx-relay     │◄──►│ Nostr       │
│ Node        │    │ Library      │    │ Relay       │
│ (RPC)       │    │              │    │ (WebSocket) │
└─────────────┘    └──────────────┘    └─────────────┘
```

## Error Handling

The library provides comprehensive error types:

```rust
use bitcoin_nostr_relay::ValidationError;

match relay.validate_transaction(tx_hex).await {
    Ok(()) => println!("Transaction valid"),
    Err(ValidationError::EmptyTransaction) => println!("Empty transaction"),
    Err(ValidationError::InvalidHex) => println!("Invalid hex format"),
    Err(ValidationError::InvalidSize(size)) => println!("Invalid size: {} bytes", size),
    Err(ValidationError::RecentlyProcessed(txid)) => println!("Recently processed: {}", txid),
    Err(ValidationError::BitcoinCoreRejection(reason)) => println!("Rejected: {}", reason),
    Err(e) => println!("Other error: {}", e),
}
```

## Testing

Run the test suite:

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration_tests

# All tests including ignored integration tests
cargo test -- --include-ignored
```

The library includes:
- **47 unit tests** covering all core functionality
- **7 integration tests** for end-to-end scenarios
- **Ignored tests** for scenarios requiring external services (Bitcoin Core, Nostr relays)

## Examples

See the [playground repository](https://github.com/vnprc/tx-relay-playground) for complete examples including:
- Multi-relay setup with nix
- Bitcoin regtest and testnet4 configurations
- End-to-end transaction relay demonstrations

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