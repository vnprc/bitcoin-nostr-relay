use crate::{BitcoinRpcClient, NostrClient, TransactionValidator, ValidationError};
use super::config::RelayConfig;

use anyhow::Result;
use bitcoin::{consensus::deserialize, Transaction};
use futures_util::{SinkExt, StreamExt};
use nostr::{Event, EventBuilder, Keys, Kind, Tag};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};
use url::Url;

// Transaction relay event kinds
const KIND_SUBMIT_TX: u16 = 20010;
const KIND_TX_RESPONSE: u16 = 20011;  
const KIND_TX_BROADCAST: u16 = 20012;
const KIND_REQUEST_TX: u16 = 20013;

type ClientMap = Arc<RwLock<HashMap<String, broadcast::Sender<Event>>>>;

/// Core Bitcoin-Nostr relay server implementation
#[derive(Clone)]
pub struct RelayServer {
    bitcoin_client: BitcoinRpcClient,
    clients: ClientMap,
    keys: Keys,
    tx_broadcaster: broadcast::Sender<Event>,
    strfry_sender: mpsc::UnboundedSender<Event>,
    strfry_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Event>>>,
    remote_transactions: Arc<RwLock<HashSet<String>>>,
    validator: TransactionValidator,
    config: RelayConfig,
}

impl RelayServer {
    /// Create a new RelayServer with the given components
    pub fn new(
        bitcoin_client: BitcoinRpcClient,
        _nostr_client: Option<NostrClient>,
        validator: TransactionValidator,
        config: RelayConfig,
    ) -> Result<Self> {
        let (tx_broadcaster, _) = broadcast::channel(1000);
        let (strfry_sender, strfry_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            bitcoin_client,
            clients: Arc::new(RwLock::new(HashMap::new())),
            keys: Keys::generate(),
            tx_broadcaster,
            strfry_sender,
            strfry_receiver: Arc::new(tokio::sync::Mutex::new(strfry_receiver)),
            remote_transactions: Arc::new(RwLock::new(HashSet::new())),
            validator,
            config,
        })
    }
    
    /// Start the relay server on the given address
    pub async fn run(self) -> Result<()> {
        let addr = self.config.websocket_listen_addr;
        let listener = TcpListener::bind(addr).await?;
        info!("Relay-{} Bitcoin Transaction Relay Server listening on {}", self.config.relay_id, addr);
        
        // Start mempool monitoring task
        let server_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = server_clone.monitor_mempool().await {
                error!("Relay-{}: Mempool monitoring error: {}", server_clone.config.relay_id, e);
            }
        });
        
        // Start strfry client connection task
        let server_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = server_clone.connect_to_strfry().await {
                error!("Relay-{}: Strfry connection error: {}", server_clone.config.relay_id, e);
            }
        });
        
        while let Ok((stream, peer_addr)) = listener.accept().await {
            info!("New client connection from {}", peer_addr);
            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream, peer_addr).await {
                    error!("Error handling connection from {}: {}", peer_addr, e);
                }
            });
        }
        
        Ok(())
    }
    
    /// Handle a new WebSocket client connection
    async fn handle_connection(&self, stream: TcpStream, peer_addr: SocketAddr) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let client_id = peer_addr.to_string();
        
        let (tx_sender, mut tx_receiver) = broadcast::channel(self.config.websocket_buffer_size);
        self.clients.write().await.insert(client_id.clone(), tx_sender);
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let server = self.clone();
        
        // Handle outgoing messages to client
        let broadcast_task = tokio::spawn(async move {
            while let Ok(event) = tx_receiver.recv().await {
                let message = json!(["EVENT", "sub_id", event]).to_string();
                if let Err(e) = ws_sender.send(Message::Text(message)).await {
                    error!("Failed to send message to client: {}", e);
                    break;
                }
            }
        });
        
        // Handle incoming messages from client
        while let Some(msg) = ws_receiver.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Err(e) = server.handle_nostr_message(&text, &client_id).await {
                        error!("Error handling nostr message: {}", e);
                    }
                }
                Message::Close(_) => {
                    info!("Client {} disconnected", client_id);
                    break;
                }
                _ => {}
            }
        }
        
        broadcast_task.abort();
        self.clients.write().await.remove(&client_id);
        Ok(())
    }
    
    /// Handle incoming Nostr messages from clients
    async fn handle_nostr_message(&self, message: &str, client_id: &str) -> Result<()> {
        let parsed: Value = serde_json::from_str(message)?;
        
        if let Some(arr) = parsed.as_array() {
            if arr.len() >= 2 {
                let msg_type = arr[0].as_str().unwrap_or("");
                
                match msg_type {
                    "EVENT" => {
                        if arr.len() >= 2 {
                            let event: Event = serde_json::from_value(arr[1].clone())?;
                            self.handle_event(event, client_id).await?;
                        }
                    }
                    "REQ" => {
                        info!("Client {} subscribed", client_id);
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle specific Nostr events
    async fn handle_event(&self, event: Event, client_id: &str) -> Result<()> {
        let kind = event.kind.as_u32();
        match kind {
            k if k == KIND_SUBMIT_TX as u32 => self.handle_submit_tx(event, client_id).await,
            k if k == KIND_REQUEST_TX as u32 => self.handle_request_tx(event, client_id).await,
            _ => {
                warn!("Unhandled event kind: {}", event.kind.as_u32());
                Ok(())
            }
        }
    }
    
    /// Handle transaction submission from clients
    async fn handle_submit_tx(&self, event: Event, client_id: &str) -> Result<()> {
        info!("🌐 Relay-{}: Received transaction via WEBSOCKET from {}", self.config.relay_id, client_id);
        
        let tx_hex = event.content.trim();
        
        // Validate transaction
        match self.validator.validate(tx_hex).await {
            Ok(()) => {
                // Validation passed, continue to submission
            }
            Err(ValidationError::RecentlyProcessed(_)) => {
                self.send_tx_response(client_id, false, "Transaction recently processed", "").await?;
                return Ok(());
            }
            Err(e) => {
                self.send_tx_response(client_id, false, &e.to_string(), "").await?;
                return Ok(());
            }
        }
        
        // Decode and process transaction
        match hex::decode(tx_hex) {
            Ok(tx_bytes) => {
                match deserialize::<Transaction>(&tx_bytes) {
                    Ok(tx) => {
                        let txid = tx.txid().to_string();
                        info!("Decoded transaction: {}", txid);
                        
                        match self.submit_to_bitcoin_node(tx_hex).await {
                            Ok(_) => {
                                self.send_tx_response(client_id, true, "Transaction accepted", &txid).await?;
                            }
                            Err(e) => {
                                error!("Failed to submit transaction to Bitcoin node: {}", e);
                                self.send_tx_response(client_id, false, &e.to_string(), &txid).await?;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize transaction: {}", e);
                        self.send_tx_response(client_id, false, "Invalid transaction format", "").await?;
                    }
                }
            }
            Err(e) => {
                error!("Failed to decode transaction hex: {}", e);
                self.send_tx_response(client_id, false, "Invalid hex encoding", "").await?;
            }
        }
        
        Ok(())
    }
    
    /// Submit a transaction to the Bitcoin node
    async fn submit_to_bitcoin_node(&self, tx_hex: &str) -> Result<String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendrawtransaction",
            "params": [tx_hex]
        });
        
        let client = reqwest::Client::new();
        let response = client
            .post(&self.config.bitcoin_rpc_url)
            .basic_auth(&self.config.bitcoin_rpc_auth.username, Some(&self.config.bitcoin_rpc_auth.password))
            .json(&request)
            .send()
            .await?
            .json::<Value>()
            .await?;
        
        if let Some(error) = response.get("error") {
            if !error.is_null() {
                return Err(anyhow::anyhow!("Bitcoin RPC error: {}", error));
            }
        }
        
        let txid = response["result"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No txid in response"))?
            .to_string();
        
        Ok(txid)
    }
    
    /// Send a transaction response back to the client
    async fn send_tx_response(&self, client_id: &str, success: bool, message: &str, txid: &str) -> Result<()> {
        let content = json!({
            "success": success,
            "message": message,
            "txid": txid
        });
        
        let event = EventBuilder::new(
            Kind::Ephemeral(KIND_TX_RESPONSE),
            content.to_string(),
            &[]
        ).to_event(&self.keys)?;
        
        if let Some(sender) = self.clients.read().await.get(client_id) {
            let _ = sender.send(event);
        }
        
        Ok(())
    }
    
    /// Handle transaction lookup requests
    async fn handle_request_tx(&self, _event: Event, client_id: &str) -> Result<()> {
        info!("Transaction request from client {}", client_id);
        // TODO: Implement transaction lookup
        Ok(())
    }
    
    /// Monitor the Bitcoin mempool for new transactions
    async fn monitor_mempool(&self) -> Result<()> {
        let mut known_txids = match self.get_mempool_txids().await {
            Ok(txids) => {
                info!("Relay-{}: Initialized with {} existing transactions in mempool", self.config.relay_id, txids.len());
                txids.into_iter().collect()
            }
            Err(e) => {
                warn!("Relay-{}: Failed to get initial mempool state: {}, starting with empty set", self.config.relay_id, e);
                std::collections::HashSet::new()
            }
        };
        
        info!("Relay-{}: Starting mempool monitoring", self.config.relay_id);
        
        loop {
            match self.get_mempool_txids().await {
                Ok(current_txids) => {
                    for txid in &current_txids {
                        if !known_txids.contains(txid) {
                            let is_remote = {
                                let remote_txs = self.remote_transactions.read().await;
                                remote_txs.contains(txid)
                            };
                            
                            if !is_remote {
                                if let Ok(raw_tx) = self.get_raw_transaction(txid).await {
                                    if let Ok(tx) = bitcoin::consensus::deserialize::<bitcoin::Transaction>(
                                        &hex::decode(&raw_tx)?
                                    ) {
                                        if let Err(e) = self.broadcast_transaction(&tx, txid).await {
                                            error!("Relay-{}: Failed to broadcast transaction {}: {}", self.config.relay_id, txid, e);
                                        }
                                    }
                                }
                            }
                            
                            known_txids.insert(txid.clone());
                        }
                    }
                    
                    known_txids.retain(|txid| current_txids.contains(txid));
                }
                Err(e) => {
                    error!("Relay-{}: Failed to get mempool: {}", self.config.relay_id, e);
                }
            }
            
            tokio::time::sleep(self.config.mempool_poll_interval).await;
        }
    }
    
    /// Get the list of transaction IDs from the mempool
    async fn get_mempool_txids(&self) -> Result<Vec<String>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getrawmempool",
            "params": []
        });
        
        let client = reqwest::Client::new();
        let response = client
            .post(&self.config.bitcoin_rpc_url)
            .basic_auth(&self.config.bitcoin_rpc_auth.username, Some(&self.config.bitcoin_rpc_auth.password))
            .json(&request)
            .send()
            .await?
            .json::<Value>()
            .await?;
        
        if let Some(error) = response.get("error") {
            if !error.is_null() {
                return Err(anyhow::anyhow!("Bitcoin RPC error: {}", error));
            }
        }
        
        let txids: Vec<String> = response["result"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();
            
        Ok(txids)
    }
    
    /// Get the raw transaction hex for a given transaction ID
    async fn get_raw_transaction(&self, txid: &str) -> Result<String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getrawtransaction",
            "params": [txid]
        });
        
        let client = reqwest::Client::new();
        let response = client
            .post(&self.config.bitcoin_rpc_url)
            .basic_auth(&self.config.bitcoin_rpc_auth.username, Some(&self.config.bitcoin_rpc_auth.password))
            .json(&request)
            .send()
            .await?
            .json::<Value>()
            .await?;
        
        if let Some(error) = response.get("error") {
            if !error.is_null() {
                return Err(anyhow::anyhow!("Bitcoin RPC error: {}", error));
            }
        }
        
        Ok(response["result"].as_str().unwrap_or("").to_string())
    }
    
    /// Broadcast a transaction to the Nostr network
    async fn broadcast_transaction(&self, tx: &Transaction, txid: &str) -> Result<()> {
        let content = json!({
            "txid": txid,
            "size": bitcoin::consensus::serialize(tx).len(),
            "version": tx.version,
            "inputs": tx.input.len(),
            "outputs": tx.output.len(),
            "hex": hex::encode(bitcoin::consensus::serialize(tx))
        });
        
        let event = EventBuilder::new(
            Kind::Ephemeral(KIND_TX_BROADCAST), 
            content.to_string(),
            &[
                Tag::Hashtag("bitcoin".to_string()),
                Tag::Hashtag("transaction".to_string()),
                Tag::Generic(
                    nostr::TagKind::Custom("relay_id".to_string()),
                    vec![self.config.relay_id.clone()],
                ),
            ]
        ).to_event(&self.keys)?;
        
        match self.send_to_strfry(&event).await {
            Ok(_) => info!("📡 Relay-{}: Broadcasting transaction {} via Nostr", self.config.relay_id, txid),
            Err(e) => error!("Relay-{}: Failed to broadcast transaction {} to strfry: {}", self.config.relay_id, txid, e),
        }
        
        let clients = self.clients.read().await;
        for sender in clients.values() {
            let _ = sender.send(event.clone());
        }
        
        Ok(())
    }
    
    /// Send an event to the Strfry relay
    async fn send_to_strfry(&self, event: &Event) -> Result<()> {
        if let Err(_) = self.strfry_sender.send(event.clone()) {
            return Err(anyhow::anyhow!("Failed to send event to strfry channel"));
        }
        Ok(())
    }
    
    /// Connect to the Strfry Nostr relay
    async fn connect_to_strfry(&self) -> Result<()> {
        info!("Relay-{}: Connecting to strfry relay at {}", self.config.relay_id, self.config.strfry_url);
        
        loop {
            match self.try_connect_to_strfry().await {
                Ok(_) => {
                    info!("Relay-{}: Strfry connection closed, reconnecting in 5 seconds", self.config.relay_id);
                }
                Err(e) => {
                    error!("Relay-{}: Failed to connect to strfry: {}, retrying in 5 seconds", self.config.relay_id, e);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
    
    /// Attempt to connect to Strfry (with retry logic)
    async fn try_connect_to_strfry(&self) -> Result<()> {
        let url = Url::parse(&self.config.strfry_url)?;
        let (ws_stream, _) = connect_async(url).await?;
        info!("Relay-{}: Connected to strfry relay", self.config.relay_id);
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Subscribe to transaction broadcasts
        let current_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let subscription = json!([
            "REQ",
            format!("tx_relay_{}", self.config.relay_id),
            {
                "kinds": [KIND_TX_BROADCAST as u64],
                "#t": ["bitcoin", "transaction"],
                "since": current_timestamp
            }
        ]);
        
        ws_sender.send(Message::Text(subscription.to_string())).await?;
        info!("Relay-{}: Subscribed to transaction broadcasts", self.config.relay_id);
        
        let strfry_receiver = Arc::clone(&self.strfry_receiver);
        let mut strfry_receiver = strfry_receiver.lock().await;
        
        loop {
            tokio::select! {
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_strfry_message(&text).await {
                                error!("Relay-{}: Error handling strfry message: {}", self.config.relay_id, e);
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Relay-{}: Strfry connection closed", self.config.relay_id);
                            break;
                        }
                        Some(Err(e)) => {
                            error!("Relay-{}: WebSocket error: {}", self.config.relay_id, e);
                            break;
                        }
                        None => break,
                        _ => {}
                    }
                }
                event = strfry_receiver.recv() => {
                    if let Some(event) = event {
                        let message = json!(["EVENT", event]);
                        if let Err(e) = ws_sender.send(Message::Text(message.to_string())).await {
                            error!("Relay-{}: Failed to send event to strfry: {}", self.config.relay_id, e);
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle messages received from the Strfry relay
    async fn handle_strfry_message(&self, message: &str) -> Result<()> {
        let parsed: Value = serde_json::from_str(message)?;
        
        if let Some(arr) = parsed.as_array() {
            if arr.len() >= 3 && arr[0].as_str() == Some("EVENT") {
                let event: Event = serde_json::from_value(arr[2].clone())?;
                
                if event.kind.as_u32() == KIND_TX_BROADCAST as u32 {
                    self.handle_remote_transaction(event).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle transactions received from remote relays
    async fn handle_remote_transaction(&self, event: Event) -> Result<()> {
        // Check if this event came from our own relay
        for tag in &event.tags {
            if let nostr::Tag::Generic(kind, values) = tag {
                if *kind == nostr::TagKind::Custom("relay_id".to_string()) && !values.is_empty() {
                    if values[0] == self.config.relay_id {
                        return Ok(());
                    }
                }
            }
        }
        
        let tx_data: Value = serde_json::from_str(&event.content)?;
        
        if let Some(tx_hex) = tx_data.get("hex").and_then(|h| h.as_str()) {
            if let Some(txid) = tx_data.get("txid").and_then(|t| t.as_str()) {
                let mut remote_txs = self.remote_transactions.write().await;
                remote_txs.insert(txid.to_string());
                
                match self.validator.validate(tx_hex).await {
                    Ok(()) => {}
                    Err(ValidationError::RecentlyProcessed(_)) => {
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Relay-{}: Transaction {} failed validation: {}", self.config.relay_id, txid, e);
                        return Ok(());
                    }
                }
                
                match self.submit_to_bitcoin_node(tx_hex).await {
                    Ok(_) => {
                        info!("🌐 Relay-{}: Received transaction {} via Nostr", self.config.relay_id, txid);
                    }
                    Err(e) => {
                        if !e.to_string().contains("already in mempool") && !e.to_string().contains("already exists") {
                            warn!("Relay-{}: Failed to submit remote transaction {} to local Bitcoin node: {}", self.config.relay_id, txid, e);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}