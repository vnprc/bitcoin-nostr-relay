use crate::relay::RelayConfig;
use std::net::SocketAddr;

/// Common Bitcoin network types for convenient relay configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Regtest,
    Testnet4,
}

/// Generate configuration for common network patterns
/// 
/// This function provides the convenience layer mentioned in the migration plan,
/// allowing users to quickly configure relays for common scenarios while still
/// using the explicit configuration API underneath.
pub fn network_config(network: Network, relay_id: u16) -> RelayConfig {
    let (bitcoin_port, websocket_port, strfry_port) = match (network, relay_id) {
        (Network::Regtest, 1) => (18332, 7779, 7777),
        (Network::Regtest, 2) => (18444, 7780, 7778),
        (Network::Testnet4, 1) => (48330, 7779, 7777),
        (Network::Testnet4, 2) => (48350, 7780, 7778),
        _ => panic!("Unsupported configuration: {:?} with relay_id {}", network, relay_id),
    };
    
    RelayConfig::new(
        format!("http://127.0.0.1:{}", bitcoin_port),
        format!("ws://127.0.0.1:{}", strfry_port),
        relay_id.to_string(),
        SocketAddr::from(([127, 0, 0, 1], websocket_port)),
    ).expect("Hardcoded network configuration should always be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_regtest() {
        let config1 = network_config(Network::Regtest, 1);
        assert_eq!(config1.bitcoin_rpc_url, "http://127.0.0.1:18332");
        assert_eq!(config1.strfry_url, "ws://127.0.0.1:7777");
        assert_eq!(config1.websocket_listen_addr, "127.0.0.1:7779".parse::<SocketAddr>().unwrap());
        assert_eq!(config1.relay_id, "1");
        
        let config2 = network_config(Network::Regtest, 2);
        assert_eq!(config2.bitcoin_rpc_url, "http://127.0.0.1:18444");
        assert_eq!(config2.strfry_url, "ws://127.0.0.1:7778");
        assert_eq!(config2.websocket_listen_addr, "127.0.0.1:7780".parse::<SocketAddr>().unwrap());
        assert_eq!(config2.relay_id, "2");
    }

    #[test]
    fn test_network_config_testnet4() {
        let config1 = network_config(Network::Testnet4, 1);
        assert_eq!(config1.bitcoin_rpc_url, "http://127.0.0.1:48330");
        assert_eq!(config1.strfry_url, "ws://127.0.0.1:7777");
        assert_eq!(config1.websocket_listen_addr, "127.0.0.1:7779".parse::<SocketAddr>().unwrap());
        assert_eq!(config1.relay_id, "1");
        
        let config2 = network_config(Network::Testnet4, 2);
        assert_eq!(config2.bitcoin_rpc_url, "http://127.0.0.1:48350");
        assert_eq!(config2.strfry_url, "ws://127.0.0.1:7778");
        assert_eq!(config2.websocket_listen_addr, "127.0.0.1:7780".parse::<SocketAddr>().unwrap());
        assert_eq!(config2.relay_id, "2");
    }

    #[test]
    fn test_network_config_builder_pattern() {
        let config = network_config(Network::Regtest, 1)
            .with_auth("custom_user".to_string(), "custom_pass".to_string())
            .with_mempool_poll_interval_secs(5);
            
        assert_eq!(config.bitcoin_rpc_auth.username, "custom_user");
        assert_eq!(config.bitcoin_rpc_auth.password, "custom_pass");
        assert_eq!(config.mempool_poll_interval.as_secs(), 5);
    }

    #[test]
    #[should_panic(expected = "Unsupported configuration")]
    fn test_network_config_unsupported() {
        network_config(Network::Regtest, 99); // Should panic
    }
}