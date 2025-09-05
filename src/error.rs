use thiserror::Error;

/// Main error type for the bitcoin-nostr-relay library
#[derive(Error, Debug)]
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
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    
    #[error("Bitcoin error: {0}")]
    Bitcoin(#[from] bitcoin::consensus::encode::Error),
    
    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
    
    #[error("Nostr key error: {0}")]
    NostrKey(#[from] nostr::key::Error),
    
    #[error("Nostr event error: {0}")]
    NostrEvent(#[from] nostr::event::Error),
    
    #[error("Nostr event builder error: {0}")]
    NostrEventBuilder(#[from] nostr::event::builder::Error),
    
    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    
    #[error("{0}")]
    Other(String),
}

/// Configuration-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid URL: {url}")]
    InvalidUrl { url: String },
    
    #[error("Invalid socket address: {addr}")]
    InvalidSocketAddr { addr: String },
    
    #[error("Unsupported network configuration: {network:?} with relay_id {relay_id}")]
    UnsupportedConfiguration { network: crate::Network, relay_id: u16 },
    
    #[error("Invalid authentication credentials")]
    InvalidAuth,
    
    #[error("Invalid configuration parameter: {param}")]
    InvalidParameter { param: String },
}

/// Bitcoin RPC-specific errors
#[derive(Error, Debug)]
pub enum BitcoinRpcError {
    #[error("RPC request failed: {message}")]
    RequestFailed { message: String },
    
    #[error("Invalid RPC response format")]
    InvalidResponse,
    
    #[error("Connection failed to {url}")]
    ConnectionFailed { url: String },
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Bitcoin Core error: {code} - {message}")]
    BitcoinCore { code: i32, message: String },
}

/// Nostr-specific errors  
#[derive(Error, Debug)]
pub enum NostrError {
    #[error("Failed to connect to Nostr relay: {url}")]
    ConnectionFailed { url: String },
    
    #[error("Failed to send event to Nostr relay")]
    SendFailed,
    
    #[error("Invalid Nostr event format")]
    InvalidEvent,
    
    #[error("Nostr relay disconnected")]
    Disconnected,
    
    #[error("Subscription failed")]
    SubscriptionFailed,
}

/// Transaction validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Empty transaction")]
    EmptyTransaction,
    
    #[error("Invalid hex format")]
    InvalidHex,
    
    #[error("Invalid transaction size: {size} bytes")]
    InvalidSize { size: usize },
    
    #[error("Invalid transaction structure")]
    InvalidStructure,
    
    #[error("Transaction {txid} recently processed (cached)")]
    RecentlyProcessed { txid: String },
    
    #[error("Bitcoin Core rejection: {reason}")]
    BitcoinCoreRejection { reason: String },
    
    #[error("Validation timeout")]
    Timeout,
    
    #[error("Validation disabled")]
    Disabled,
}

// Add conversion from reqwest::Error to ValidationError for HTTP requests
impl From<reqwest::Error> for ValidationError {
    fn from(err: reqwest::Error) -> Self {
        Self::bitcoin_core_rejection(format!("HTTP error: {}", err))
    }
}

// Add conversion from serde_json::Error to ValidationError for JSON parsing
impl From<serde_json::Error> for ValidationError {
    fn from(err: serde_json::Error) -> Self {
        Self::bitcoin_core_rejection(format!("JSON error: {}", err))
    }
}

/// Network-related errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to bind to address: {addr}")]
    BindFailed { addr: std::net::SocketAddr },
    
    #[error("Client connection failed")]
    ClientConnectionFailed,
    
    #[error("WebSocket handshake failed")]
    WebSocketHandshakeFailed,
    
    #[error("Connection timeout")]
    Timeout,
    
    #[error("Connection closed unexpectedly")]
    ConnectionClosed,
    
    #[error("Maximum connections exceeded")]
    MaxConnectionsExceeded,
}

// Conversion from anyhow::Error for gradual migration
impl From<anyhow::Error> for RelayError {
    fn from(err: anyhow::Error) -> Self {
        RelayError::Other(err.to_string())
    }
}

// Helper methods for common error patterns
impl ValidationError {
    pub fn invalid_size(size: usize) -> Self {
        Self::InvalidSize { size }
    }
    
    pub fn recently_processed(txid: impl Into<String>) -> Self {
        Self::RecentlyProcessed { txid: txid.into() }
    }
    
    pub fn bitcoin_core_rejection(reason: impl Into<String>) -> Self {
        Self::BitcoinCoreRejection { reason: reason.into() }
    }
}

impl BitcoinRpcError {
    pub fn request_failed(message: impl Into<String>) -> Self {
        Self::RequestFailed { message: message.into() }
    }
    
    pub fn connection_failed(url: impl Into<String>) -> Self {
        Self::ConnectionFailed { url: url.into() }
    }
    
    pub fn bitcoin_core(code: i32, message: impl Into<String>) -> Self {
        Self::BitcoinCore { code, message: message.into() }
    }
}

impl ConfigError {
    pub fn invalid_url(url: impl Into<String>) -> Self {
        Self::InvalidUrl { url: url.into() }
    }
    
    pub fn invalid_socket_addr(addr: impl Into<String>) -> Self {
        Self::InvalidSocketAddr { addr: addr.into() }
    }
    
    pub fn unsupported_configuration(network: crate::Network, relay_id: u16) -> Self {
        Self::UnsupportedConfiguration { network, relay_id }
    }
}

impl NetworkError {
    pub fn bind_failed(addr: std::net::SocketAddr) -> Self {
        Self::BindFailed { addr }
    }
}

impl NostrError {
    pub fn connection_failed(url: impl Into<String>) -> Self {
        Self::ConnectionFailed { url: url.into() }
    }
}