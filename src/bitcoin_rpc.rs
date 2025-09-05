use crate::error::BitcoinRpcError;
use crate::Result;
use bitcoin::{Block, BlockHash};
use reqwest::Client;
use serde_json::{json, Value};
use std::str::FromStr;

#[derive(Clone)]
pub struct BitcoinRpcClient {
    client: Client,
    url: String,
    username: String,
    password: String,
}

impl BitcoinRpcClient {
    pub fn new(url: String, username: String, password: String) -> Self {
        Self {
            client: Client::new(),
            url,
            username,
            password,
        }
    }
    
    async fn rpc_call(&self, method: &str, params: &Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });
        
        let response = self
            .client
            .post(&self.url)
            .basic_auth(&self.username, Some(&self.password))
            .json(&request)
            .send()
            .await?
            .json::<Value>()
            .await?;
        
        if let Some(error) = response.get("error") {
            if !error.is_null() {
                return Err(BitcoinRpcError::request_failed(format!("RPC error: {}", error)).into());
            }
        }
        
        response
            .get("result")
            .cloned()
            .ok_or_else(|| BitcoinRpcError::InvalidResponse.into())
    }
    
    pub async fn get_best_block_hash(&self) -> Result<BlockHash> {
        let result = self.rpc_call("getbestblockhash", &json!([])).await?;
        let hash_str = result
            .as_str()
            .ok_or_else(|| BitcoinRpcError::InvalidResponse)?;
        BlockHash::from_str(hash_str).map_err(|e| BitcoinRpcError::request_failed(format!("Failed to parse block hash: {}", e)).into())
    }
    
    pub async fn get_block(&self, block_hash: &BlockHash) -> Result<Block> {
        let result = self
            .rpc_call("getblock", &json!([block_hash.to_string(), 0]))
            .await?;
        let block_hex = result
            .as_str()
            .ok_or_else(|| BitcoinRpcError::InvalidResponse)?;
        let block_bytes = hex::decode(block_hex)?;
        bitcoin::consensus::deserialize(&block_bytes)
            .map_err(|e| BitcoinRpcError::request_failed(format!("Failed to deserialize block: {}", e)).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use bitcoin::hashes::Hash;

    #[test]
    fn test_bitcoin_rpc_client_creation() {
        let client = BitcoinRpcClient::new(
            "http://127.0.0.1:18332".to_string(),
            "testuser".to_string(),
            "testpassword".to_string(),
        );
        
        assert_eq!(client.url, "http://127.0.0.1:18332");
        assert_eq!(client.username, "testuser");
        assert_eq!(client.password, "testpassword");
    }

    #[test]
    fn test_bitcoin_rpc_client_clone() {
        let client1 = BitcoinRpcClient::new(
            "http://127.0.0.1:18332".to_string(),
            "testuser".to_string(),
            "testpassword".to_string(),
        );
        
        let client2 = client1.clone();
        assert_eq!(client1.url, client2.url);
        assert_eq!(client1.username, client2.username);
        assert_eq!(client1.password, client2.password);
    }

    // Integration tests that require a running Bitcoin node
    #[tokio::test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    async fn test_get_best_block_hash_integration() {
        let client = BitcoinRpcClient::new(
            "http://127.0.0.1:18332".to_string(),
            "user".to_string(),
            "password".to_string(),
        );
        
        let result = client.get_best_block_hash().await;
        // This test requires actual Bitcoin Core running
        match result {
            Ok(hash) => {
                // BlockHash should be valid
                assert!(!hash.to_string().is_empty());
                assert_eq!(hash.to_string().len(), 64); // Bitcoin block hashes are 64 hex chars
            }
            Err(e) => {
                // If Bitcoin Core is not running, we expect a connection error
                assert!(e.to_string().contains("Connection") || e.to_string().contains("refused"));
            }
        }
    }

    #[tokio::test]
    #[ignore] // Use `cargo test -- --ignored` to run this test
    async fn test_get_block_integration() {
        let client = BitcoinRpcClient::new(
            "http://127.0.0.1:18332".to_string(),
            "user".to_string(),
            "password".to_string(),
        );
        
        // Use a known genesis block hash for regtest
        let genesis_hash = BlockHash::from_str("0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206")
            .expect("Valid genesis hash");
        
        let result = client.get_block(&genesis_hash).await;
        match result {
            Ok(block) => {
                // Genesis block should have specific properties
                assert_eq!(block.header.prev_blockhash, BlockHash::from_byte_array([0; 32]));
                assert_eq!(block.txdata.len(), 1); // Genesis block has one transaction
            }
            Err(e) => {
                // If Bitcoin Core is not running, we expect a connection error
                assert!(e.to_string().contains("Connection") || e.to_string().contains("refused"));
            }
        }
    }

    #[test]
    fn test_block_hash_parsing() {
        // Test valid block hash parsing
        let valid_hash_str = "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206";
        let hash = BlockHash::from_str(valid_hash_str);
        assert!(hash.is_ok());
        
        // Test invalid block hash parsing
        let invalid_hash_str = "invalid_hash";
        let hash = BlockHash::from_str(invalid_hash_str);
        assert!(hash.is_err());
    }
}