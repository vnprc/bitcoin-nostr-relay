use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use nostr::{Event, EventBuilder, Keys, Kind, Tag};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream, MaybeTlsStream};
use tracing::{info, warn};

pub struct NostrClient {
    ws_stream: Arc<Mutex<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>>,
    keys: Keys,
}

impl NostrClient {
    pub fn new(ws_stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>) -> Self {
        // Generate random keys for demonstration - in production, use persistent keys
        let keys = Keys::generate();
        
        Self {
            ws_stream: Arc::new(Mutex::new(ws_stream)),
            keys,
        }
    }
    
    pub async fn send_tx_event(&self, content: &str, block_hash: &str) -> Result<()> {
        // Create bitcoin transaction event (ephemeral)
        let event = EventBuilder::new(
            Kind::Ephemeral(20001), // Bitcoin transaction kind
            content,
            &[
                Tag::Hashtag("bitcoin".to_string()),
                Tag::Hashtag("transaction".to_string()),
                Tag::Generic(
                    nostr::TagKind::Custom("block".to_string()),
                    vec![block_hash.to_string()]
                ),
            ]
        )
        .to_event(&self.keys)?;
        
        self.send_event(event).await
    }
    
    pub async fn send_event(&self, event: Event) -> Result<()> {
        let message = serde_json::to_string(&serde_json::json!(["EVENT", event]))?;
        info!("Sending nostr event: {}", event.id);
        
        let mut ws = self.ws_stream.lock().await;
        ws.send(Message::Text(message)).await?;
        
        // Try to read response (non-blocking)
        if let Some(msg) = ws.next().await {
            match msg? {
                Message::Text(text) => {
                    info!("Nostr relay response: {}", text);
                }
                Message::Binary(_) => {
                    warn!("Received binary message from nostr relay");
                }
                Message::Close(_) => {
                    warn!("Nostr relay closed connection");
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::prelude::SecretKey;
    
    #[test]
    fn test_nostr_client_creation() {
        // Test that we can create a NostrClient with generated keys
        // We can't easily test the actual WebSocket functionality without mocking
        let keys = Keys::generate();
        
        // Test key generation produces valid keys
        assert!(!keys.public_key().to_string().is_empty());
        assert!(!keys.secret_key().unwrap().secret_bytes().is_empty());
    }
    
    #[test]
    fn test_keys_generation_deterministic() {
        // Test that the same secret key produces the same public key
        let secret_bytes = [1u8; 32];
        let keys1 = Keys::new(SecretKey::from_slice(&secret_bytes).unwrap());
        let keys2 = Keys::new(SecretKey::from_slice(&secret_bytes).unwrap());
        
        assert_eq!(keys1.public_key(), keys2.public_key());
    }
    
    #[test]
    fn test_event_creation() {
        let keys = Keys::generate();
        
        // Test creating a bitcoin transaction event
        let content = "test transaction content";
        let block_hash = "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f";
        
        let event = EventBuilder::new(
            Kind::Ephemeral(20001),
            content,
            &[
                Tag::Hashtag("bitcoin".to_string()),
                Tag::Hashtag("transaction".to_string()),
                Tag::Generic(
                    nostr::TagKind::Custom("block".to_string()),
                    vec![block_hash.to_string()]
                ),
            ]
        )
        .to_event(&keys);
        
        assert!(event.is_ok());
        let event = event.unwrap();
        
        // Verify event properties
        assert_eq!(event.kind.as_u32(), 20001);
        assert_eq!(event.content, content);
        assert_eq!(event.tags.len(), 3);
        
        // Verify tags
        let hashtag_count = event.tags.iter()
            .filter(|tag| matches!(tag, nostr::Tag::Hashtag(_)))
            .count();
        assert_eq!(hashtag_count, 2);
        
        // Verify block tag
        let has_block_tag = event.tags.iter()
            .any(|tag| match tag {
                nostr::Tag::Generic(kind, values) => {
                    *kind == nostr::TagKind::Custom("block".to_string()) && 
                    values.len() == 1 && 
                    values[0] == block_hash
                }
                _ => false
            });
        assert!(has_block_tag);
    }
    
    #[test]
    fn test_event_serialization() {
        let keys = Keys::generate();
        let content = "test content";
        
        let event = EventBuilder::new(
            Kind::Ephemeral(20001),
            content,
            &[Tag::Hashtag("test".to_string())]
        )
        .to_event(&keys)
        .unwrap();
        
        // Test that event can be serialized to JSON
        let serialized = serde_json::to_string(&event);
        assert!(serialized.is_ok());
        
        let json_str = serialized.unwrap();
        assert!(!json_str.is_empty());
        assert!(json_str.contains("\"content\":\"test content\""));
        assert!(json_str.contains("\"kind\":20001"));
    }
    
    #[test]
    fn test_nostr_message_format() {
        let keys = Keys::generate();
        let event = EventBuilder::new(
            Kind::Ephemeral(20001),
            "test",
            &[]
        )
        .to_event(&keys)
        .unwrap();
        
        // Test the message format that would be sent to relay
        let message = serde_json::json!(["EVENT", event]);
        let message_str = serde_json::to_string(&message).unwrap();
        
        assert!(message_str.starts_with("[\"EVENT\","));
        assert!(message_str.contains("\"kind\":20001"));
        assert!(message_str.contains("\"content\":\"test\""));
    }
    
    // Integration test that would require a real WebSocket connection
    #[tokio::test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    async fn test_send_tx_event_integration() {
        // This test would require setting up a real Nostr relay connection
        // For now, we'll skip it in regular test runs
        
        // In a real integration test, you would:
        // 1. Set up a test Nostr relay (like strfry in test mode)
        // 2. Connect to it
        // 3. Send a transaction event
        // 4. Verify it was received
        
        // Example structure:
        // let url = "ws://localhost:7777";
        // let (ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        // let client = NostrClient::new(ws_stream);
        // let result = client.send_tx_event("deadbeef", "block_hash").await;
        // assert!(result.is_ok());
    }
}