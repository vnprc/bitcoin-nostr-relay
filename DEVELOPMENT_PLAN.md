# bitcoin-nostr-relay Development Plan

## Executive Summary

This document outlines a comprehensive development plan for enhancing the `bitcoin-nostr-relay` library to follow mature Rust library best practices. After analyzing the codebase, the library already demonstrates excellent architectural patterns but has opportunities for improvement in error handling, API design, testing, and developer experience.

## Current State Analysis

### Library Statistics
- **Total lines of code**: ~1,892 (excluding generated/target)
- **Test coverage**: 45/52 tests passing (86.5%, 7 ignored integration tests)
- **Module organization**: Well-structured (6 core modules)
- **Dependencies**: 13 runtime dependencies (reasonable)

### Current Strengths ✅

#### 1. **Configuration Architecture**
- ✅ Builder pattern following tokio/reqwest conventions
- ✅ Deployment-agnostic URL-based configuration
- ✅ Multiple API styles (method-based, functional, explicit)
- ✅ Proper use of `SocketAddr` for network addresses

#### 2. **Project Structure**
- ✅ Clean modular organization (`src/lib.rs`: 290 lines, `src/relay/*`: 878 total lines)
- ✅ Proper re-exports in lib.rs
- ✅ Separation of concerns (config, server, validation, networking)

#### 3. **Testing Coverage**
- ✅ 52 total tests with comprehensive unit test coverage
- ✅ Integration tests with proper `#[ignore]` for external dependencies
- ✅ Multiple test scenarios covering configuration patterns

#### 4. **Documentation**
- ✅ Comprehensive README with examples and architecture diagrams
- ✅ Working configuration example (`examples/configuration_example.rs`)
- ✅ Clear API documentation with usage patterns

## Development Roadmap

### Phase 1: Error Handling & API Refinement (High Priority)

#### 1.1 Custom Error Types (High Impact, Medium Effort)
**Target**: Replace `anyhow::Result` with structured error types

**Implementation**:
```rust
// Create src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum RelayError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Bitcoin RPC error: {0}")]  
    BitcoinRpc(#[from] BitcoinRpcError),
    
    #[error("Nostr error: {0}")]
    Nostr(#[from] NostrError),
    
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Invalid URL: {url}")]
    InvalidUrl { url: String },
    
    #[error("Invalid socket address: {addr}")]
    InvalidSocketAddr { addr: String },
    
    #[error("Unsupported network configuration: {network:?} with relay_id {relay_id}")]
    UnsupportedConfiguration { network: Network, relay_id: u16 },
}
```

**Benefits**: Better error handling, easier debugging, more professional API
**Effort**: ~2-3 days
**Files to modify**: `src/lib.rs`, `src/error.rs` (new), all modules

#### 1.2 Result Type Alias (High Impact, Low Effort)
**Target**: Follow `std::io::Result<T>` pattern

**Implementation**:
```rust
// src/lib.rs
pub type Result<T, E = RelayError> = std::result::Result<T, E>;

// Usage throughout library
pub async fn start(&mut self) -> Result<()> { /* ... */ }
pub fn new(config: RelayConfig) -> Result<Self> { /* ... */ }
```

**Benefits**: Consistent error handling, cleaner API
**Effort**: ~4 hours
**Files to modify**: `src/lib.rs`, all public APIs

#### 1.3 Builder Validation (Medium Impact, Medium Effort)
**Target**: Validate during construction like `tokio::net::TcpListener::bind()`

**Implementation**:
```rust
impl RelayConfig {
    pub fn new(
        bitcoin_rpc_url: impl Into<String>,
        strfry_url: impl Into<String>, 
        relay_id: impl Into<String>,
        websocket_listen_addr: SocketAddr,
    ) -> Result<Self, ConfigError> {
        let bitcoin_url = bitcoin_rpc_url.into();
        let nostr_url = strfry_url.into();
        
        // Validate URLs during construction
        url::Url::parse(&bitcoin_url)
            .map_err(|_| ConfigError::InvalidUrl { url: bitcoin_url.clone() })?;
        url::Url::parse(&nostr_url)
            .map_err(|_| ConfigError::InvalidUrl { url: nostr_url.clone() })?;
            
        Ok(Self { /* ... */ })
    }
}
```

**Benefits**: Fail-fast error handling, better user experience
**Effort**: ~1 day
**Files to modify**: `src/relay/config.rs`, tests

### Phase 2: Documentation & Developer Experience (Medium Priority)

#### 2.1 Comprehensive Documentation (High Impact, Medium Effort)
**Target**: Add crate-level docs, rustdoc examples, doc tests

**Implementation**:
```rust
// src/lib.rs additions
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

/// High-level API for Bitcoin-over-Nostr relay functionality
/// 
/// # Examples
/// 
/// Basic usage:
/// 
/// ```
/// # tokio_test::block_on(async {
/// use bitcoin_nostr_relay::*;
/// 
/// let config = RelayConfig::for_network(Network::Regtest, 1)
///     .with_auth("user".to_string(), "password".to_string());
///     
/// let relay = BitcoinNostrRelay::new(config)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
pub struct BitcoinNostrRelay { /* ... */ }
```

**Benefits**: Professional documentation, better onboarding
**Effort**: ~2-3 days
**Files to modify**: All public APIs, add doc tests

#### 2.2 Feature Flags & Optional Dependencies (Medium Impact, High Effort)
**Target**: Make dependencies conditional like `tokio`, `serde`, `reqwest`

**Implementation**:
```toml
# Cargo.toml additions
[features]
default = ["validation", "tracing"]
validation = ["dep:bitcoin", "dep:hex"]
tracing = ["dep:tracing", "dep:tracing-subscriber"] 
server = ["dep:tokio-tungstenite", "dep:futures-util"]
rpc-client = ["dep:reqwest"]

[dependencies]
bitcoin = { version = "0.30", optional = true }
tracing = { version = "0.1", optional = true }
```

**Benefits**: Smaller binary sizes, optional functionality
**Effort**: ~3-4 days
**Files to modify**: `Cargo.toml`, all modules with conditional compilation

#### 2.3 Configuration Presets (Low Impact, Low Effort)
**Target**: Convenience constructors like `tracing_subscriber::fmt()`

**Implementation**:
```rust
impl RelayConfig {
    /// Create a configuration optimized for development
    pub fn development() -> Self {
        Self::for_network(Network::Regtest, 1)
            .with_mempool_poll_interval_secs(1) // Faster polling
    }
    
    /// Create a configuration optimized for production
    pub fn production(
        bitcoin_rpc_url: String,
        nostr_relay_url: String,
        listen_addr: SocketAddr,
    ) -> Result<Self> {
        Self::new(bitcoin_rpc_url, nostr_relay_url, "prod".to_string(), listen_addr)
            .map(|config| config.with_mempool_poll_interval_secs(10)) // Slower polling
    }
}
```

**Benefits**: Better developer experience, common use cases
**Effort**: ~1 day
**Files to modify**: `src/relay/config.rs`, documentation

### Phase 3: Testing & Quality Improvements (Lower Priority)

#### 3.1 Async Trait Abstractions (Medium Impact, Medium Effort)
**Target**: Enable better testing and mocking like `reqwest::Client`

**Implementation**:
```rust
#[async_trait::async_trait]
pub trait BitcoinRpc: Send + Sync {
    async fn get_best_block_hash(&self) -> Result<BlockHash>;
    async fn get_block(&self, hash: &BlockHash) -> Result<Block>;
}

#[async_trait::async_trait] 
impl BitcoinRpc for BitcoinRpcClient {
    // Implementation...
}

// Allows for mock implementations in tests
```

**Benefits**: Better testability, dependency injection
**Effort**: ~2-3 days
**Files to modify**: `src/bitcoin_rpc.rs`, tests, add `async-trait` dependency

#### 3.2 Enhanced Testing Suite (Medium Impact, High Effort)
**Target**: Property-based tests, benchmarks, better mocking

**Implementation**:
```toml
# Add to dev-dependencies
[dev-dependencies]
proptest = "1.0"
criterion = { version = "0.5", features = ["html_reports"] }
mockall = "0.11"
tokio-test = "0.4"
```

```rust
// Property-based testing example
use proptest::prelude::*;

proptest! {
    #[test]
    fn config_builder_never_panics(
        bitcoin_url in "http://[0-9.]{7,15}:[0-9]{4,5}",
        nostr_url in "ws://[0-9.]{7,15}:[0-9]{4,5}",
        relay_id in "[a-zA-Z0-9]{1,20}",
    ) {
        let result = RelayConfig::new(bitcoin_url, nostr_url, relay_id, "127.0.0.1:8080".parse().unwrap());
        // Should never panic, either Ok or structured Error
        prop_assert!(result.is_ok() || result.is_err());
    }
}
```

**Benefits**: Higher quality, performance insights
**Effort**: ~1-2 weeks
**Files to modify**: Add `benches/`, extensive test improvements

#### 3.3 Workspace Organization (Low Impact, Medium Effort)
**Target**: Prepare for scaling like larger projects

**Implementation**:
```toml
# Root Cargo.toml
[workspace]
members = [
    "bitcoin-nostr-relay",      # Core library
    "bitcoin-nostr-relay-cli",  # CLI tool  
    "bitcoin-nostr-relay-server", # Server binary
]
```

**Benefits**: Better organization, separate binaries
**Effort**: ~2-3 days
**Files to modify**: Restructure project layout

## Implementation Timeline

### Month 1: Core Improvements
- **Week 1**: Custom error types and Result alias
- **Week 2**: Builder validation and error handling
- **Week 3**: Documentation improvements and doc tests
- **Week 4**: Testing and integration

### Month 2: Developer Experience
- **Week 1**: Feature flags implementation
- **Week 2**: Configuration presets and convenience APIs
- **Week 3**: Async trait abstractions
- **Week 4**: Enhanced testing suite

### Month 3: Quality & Polish
- **Week 1-2**: Property-based testing and benchmarks
- **Week 3**: Workspace organization (if needed)
- **Week 4**: Final polish and release preparation

## Success Metrics

### Code Quality
- [ ] All public APIs documented with rustdoc
- [ ] Custom error types replace `anyhow` usage
- [ ] 90%+ test coverage maintained
- [ ] Zero clippy warnings on default settings
- [ ] All examples compile and run successfully

### Developer Experience
- [ ] Clear error messages with actionable feedback
- [ ] Comprehensive examples for common use cases
- [ ] Feature flags allow minimal dependencies
- [ ] Documentation includes migration guides

### API Maturity
- [ ] Follows established Rust patterns (tokio, serde, reqwest)
- [ ] Backward compatibility maintained during transitions
- [ ] Clear stability guarantees documented
- [ ] Professional error handling throughout

## Risk Assessment

### Low Risk
- Documentation improvements
- Configuration presets
- Result type alias

### Medium Risk
- Custom error types (breaking API changes)
- Feature flags (dependency management complexity)
- Builder validation (potential breaking changes)

### High Risk
- Workspace reorganization (major structural changes)
- Async trait abstractions (performance implications)

## Migration Strategy

### For Breaking Changes
1. **Deprecation Period**: Mark old APIs as deprecated
2. **Migration Guide**: Provide clear upgrade instructions
3. **Compatibility Layer**: Maintain old APIs during transition
4. **Semantic Versioning**: Follow semver strictly

### For New Features
1. **Feature Flags**: Make new functionality optional initially
2. **Documentation**: Comprehensive examples and guides
3. **Testing**: Extensive test coverage before release
4. **Community Feedback**: Gather input during development

## Conclusion

This development plan positions `bitcoin-nostr-relay` to become a mature, professional Rust library following industry best practices. The phased approach prioritizes high-impact improvements while managing risk and maintaining backward compatibility.

The library already demonstrates excellent architectural decisions, particularly in configuration design and modular structure. These improvements will enhance developer experience, code quality, and long-term maintainability without sacrificing the clean API design already achieved.