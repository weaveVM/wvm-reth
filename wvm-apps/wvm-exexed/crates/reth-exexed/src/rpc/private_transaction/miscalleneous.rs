use alloy_primitives::Bytes;
use jsonrpsee::tracing::{info, trace, warn};
use serde_json::{json, Value};
use strum::{Display, EnumIter};

#[derive(Clone, EnumIter, Display)]
pub enum BuilderKind {
    Titan,
    Beaver,
    Rsync,
}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("HTTP client error: {0}")]
    ClientError(#[from] ureq::Error),
    #[error("HTTP client error: {0}")]
    ClientErrorIo(#[from] std::io::Error),
    #[error("Invalid response from builder: {0}")]
    InvalidResponse(Value),
}

pub struct BuilderEndpoint {
    url: String,
    rpc_method: String,
}

impl BuilderEndpoint {
    fn new(url: &str, rpc_method: &str) -> Self {
        Self { url: url.to_string(), rpc_method: rpc_method.to_string() }
    }
}

impl BuilderKind {
    fn endpoint(&self) -> BuilderEndpoint {
        match self {
            BuilderKind::Titan => {
                BuilderEndpoint::new("https://rpc.titanbuilder.xyz", "eth_sendPrivateTransaction")
            }
            BuilderKind::Beaver => BuilderEndpoint::new(
                "https://mevshare-rpc.beaverbuild.org",
                "eth_sendPrivateRawTransaction",
            ),
            BuilderKind::Rsync => {
                BuilderEndpoint::new("https://rsync-builder.xyz", "eth_sendPrivateRawTransaction")
            }
        }
    }

    pub fn builder(&self) -> Result<Builder, BuilderError> {
        let endpoint = self.endpoint();

        Ok(Builder { endpoint, kind: self.clone() })
    }
}

pub struct Builder {
    endpoint: BuilderEndpoint,
    kind: BuilderKind,
}

impl Builder {
    pub async fn send_tx(&self, tx: Bytes) -> Result<(), BuilderError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": &self.endpoint.rpc_method,
            "params": [{
                "tx": tx
            }],
            "id": 1
        });

        trace!(target: "builder", ?payload, "Sending tx to builder: {}", self.kind);

        let response = ureq::post(&self.endpoint.url)
            .send_json(&payload)
            .map_err(BuilderError::ClientError)?;

        let response_body: Value = response.into_json().map_err(BuilderError::ClientErrorIo)?;

        if response_body.get("error").is_some() {
            warn!(target: "builder", ?response_body, "Builder returned an error: {}", self.kind);
            return Err(BuilderError::InvalidResponse(response_body));
        }

        info!(target: "builder", ?response_body, "Tx sent successfully to builder: {}", self.kind);
        Ok(())
    }
}
