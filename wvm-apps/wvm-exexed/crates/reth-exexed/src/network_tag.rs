use crate::constant::{WVM_ALPHANET_VERSION, WVM_DEVNET_VERSION};
use std::env;

pub enum Network {
    Devnet,
    Alphanet,
}

impl Network {
    fn get_tag(&self) -> String {
        match self {
            Network::Devnet => format!("Devnet {}", WVM_DEVNET_VERSION),
            Network::Alphanet => format!("Alphanet {}", WVM_ALPHANET_VERSION),
        }
    }
}

pub fn get_network_tag() -> String {
    let devnet_flag = env::var("DEVNET").unwrap_or("false".to_string()).to_lowercase();
    let network = if devnet_flag == "true" { Network::Devnet } else { Network::Alphanet };

    network.get_tag()
}
